//! Composters, cakes, candles and campfires.

use super::*;

/// `ComposterBlock.addItem` (`ComposterBlock.java:318-336`) for chance 1.0.
#[test]
fn clicking_cake_into_an_empty_composter_fills_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        composter_state(0),
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
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_CAKE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    let filled = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(composter_level(&mut world, at).as_deref(), Some("1"));
    assert!(
        level_events_sent(&filled)
            .iter()
            .any(|(_, event)| event.event_type
                == crate::block_behavior::composter::FILL_LEVEL_EVENT
                && event.data == 1),
        "{filled:?}"
    );
}

/// `ComposterBlock.onPlace` (`:242-245`) and `tick` (`:340-344`).
#[test]
fn a_level_seven_composter_becomes_ready_after_twenty_ticks() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        composter_state(7),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    for _ in 0..crate::block_behavior::composter::READY_DELAY {
        world.tick(Vec::new());
    }
    assert_eq!(composter_level(&mut world, at).as_deref(), Some("8"));
}

/// `ComposterBlock.extractProduce` (`ComposterBlock.java:298-309`).
#[test]
fn clicking_a_ready_composter_drops_bone_meal() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        composter_state(8),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let emptied = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(composter_level(&mut world, at).as_deref(), Some("0"));
    assert!(
        sounds_sent(&emptied).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.composter.empty")
                    .expect("a registered sound"),
            }),
        "{emptied:?}"
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
        &NbtSummaryValue::string("minecraft:bone_meal")
    );
}

/// `CakeBlock.eat` (`CakeBlock.java:94-111`) for a creative session:
/// `Player.canEat(false)` (`Player.java:1581-1583`) is true because
/// `abilities.invulnerable` is set.
#[test]
fn clicking_an_uneaten_cake_takes_a_slice() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        cake_state(0),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        crate::block_behavior::cake::analog_output(live_block(&mut world, at).unwrap()),
        14
    );
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(cake_bites(&mut world, at).as_deref(), Some("1"));
    assert_eq!(
        crate::block_behavior::cake::analog_output(live_block(&mut world, at).unwrap()),
        12
    );
    let food = live_food(&mut world);
    assert_eq!(food.food_level(), bbb_server_types::food::MAX_FOOD_LEVEL);
    assert_eq!(
        food.saturation_level(),
        bbb_server_types::food::DEFAULT_SATURATION_LEVEL
            + bbb_server_types::food::saturation_by_modifier(
                crate::block_behavior::cake::EAT_NUTRITION,
                crate::block_behavior::cake::EAT_SATURATION_MODIFIER
            )
    );
}

/// `CakeBlock.eat` (`CakeBlock.java:105-108`) on the last slice.
#[test]
fn the_seventh_cake_click_removes_the_block() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        cake_state(crate::block_behavior::cake::MAX_BITES),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(block_state_id("minecraft:air"))
    );
}

/// `Player.canEat(false)` (`Player.java:1581-1583`) refuses a full survival
/// bar: not invulnerable, and `needsFood` is false.
#[test]
fn a_full_survival_player_cannot_eat_cake() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        ..creative_fixture_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        cake_state(0),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(cake_bites(&mut world, at).as_deref(), Some("0"));
    let food = live_food(&mut world);
    assert_eq!(food.food_level(), bbb_server_types::food::MAX_FOOD_LEVEL);
    assert_eq!(
        food.saturation_level(),
        bbb_server_types::food::DEFAULT_SATURATION_LEVEL
    );
}

/// `CakeBlock.useItemOn` (`CakeBlock.java:65-71`) would place a candle cake.
/// That block is frozen, so the click is consumed and the cake is left uneaten.
#[test]
fn a_candle_on_an_uneaten_cake_does_not_eat_it() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        cake_state(0),
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
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_CANDLE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(cake_bites(&mut world, at).as_deref(), Some("0"));
}

/// `FlintAndSteelItem.useOn` lights an unlit candle (`FlintAndSteelItem.java:49-56`),
/// playing the sound with the user, whose client predicted it.
#[test]
fn flint_and_steel_use_lights_a_candle() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        candle_state(false, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:flint_and_steel");
    let lit = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(candle_state(true, false)));
    assert!(
        !sounds_sent(&lit).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:item.flintandsteel.use")
                    .expect("a registered sound"),
            }),
        "{lit:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(events.iter().any(|event| event.game_event
        == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
        && event.pos == at));
}

/// `FireChargeItem.useOn` lights an unlit campfire (`FireChargeItem.java:44-47`).
#[test]
fn fire_charge_use_lights_a_campfire() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:fire_charge");
    let lit = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(campfire_state(true)));
    assert!(
        sounds_sent(&lit).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:item.firecharge.use")
                    .expect("a registered sound"),
            }),
        "{lit:?}"
    );
}

/// `FireChargeItem.playSound` (`FireChargeItem.java:58-61`) plays with no
/// player, so the user hears the fire it places too.
#[test]
fn fire_charge_placing_fire_is_heard_by_its_user() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID
        )
        .unwrap(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:fire_charge");
    let used = click_use(&sender, &receiver, &mut world, at, 1.0, 1);
    assert_eq!(
        live_block(&mut world, BlockPos { y: 65, ..at })
            .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id),
        Some(bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID)
    );
    let heard = used.events.iter().any(|event| {
        matches!(event, WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::Sound(sound),
        } if sound.sound == SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:item.firecharge.use").unwrap(),
        })
    });
    assert!(heard, "{used:?}");
}

/// `CandleBlock.useItemOn` empty-hand douse (`CandleBlock.java:89-91`).
#[test]
fn empty_hand_use_douses_a_lit_candle() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        candle_state(true, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let doused = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(candle_state(false, false)));
    assert!(
        sounds_sent(&doused).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.candle.extinguish")
                    .expect("a registered sound"),
            }),
        "{doused:?}"
    );
}

/// `CandleCakeBlock.useWithoutItem` (`CandleCakeBlock.java:90-99`) for a
/// creative session: `CakeBlock.eat` on cake's default state, then
/// `dropResources` of the candle-cake cell. `cursor.y <= 0.5` so
/// `candleHit` (`:101-103`) does not douse first.
#[test]
fn empty_hand_use_eats_a_candle_cake_and_drops_the_candle() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        candle_cake_state(true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let _ = click_use(&sender, &receiver, &mut world, at, 0.4, 1);
    assert_eq!(live_block(&mut world, at), Some(cake_state(1)));
    let food = live_food(&mut world);
    assert_eq!(food.food_level(), bbb_server_types::food::MAX_FOOD_LEVEL);
    assert_eq!(
        food.saturation_level(),
        bbb_server_types::food::DEFAULT_SATURATION_LEVEL
            + bbb_server_types::food::saturation_by_modifier(
                crate::block_behavior::cake::EAT_NUTRITION,
                crate::block_behavior::cake::EAT_SATURATION_MODIFIER
            )
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
        &NbtSummaryValue::string("minecraft:candle")
    );
    assert_eq!(field(stack, "count"), &NbtSummaryValue::Int(1));
}

/// Full survival: `Player.canEat(false)` (`Player.java:1581-1583`) is false.
#[test]
fn a_full_survival_player_cannot_eat_candle_cake() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        candle_cake_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let _ = click_use(&sender, &receiver, &mut world, at, 0.4, 1);
    assert_eq!(live_block(&mut world, at), Some(candle_cake_state(false)));
    let food = live_food(&mut world);
    assert_eq!(food.food_level(), bbb_server_types::food::MAX_FOOD_LEVEL);
    assert_eq!(
        food.saturation_level(),
        bbb_server_types::food::DEFAULT_SATURATION_LEVEL
    );
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert!(saved_entities(&request).is_empty());
}

/// `CandleCakeBlock.useItemOn` douses only when the candle is hit (`:81-83`).
/// A creative session *can* eat; `cursor.y > 0.5` still douses first.
#[test]
fn empty_hand_use_douses_a_lit_candle_cake_only_on_the_candle() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        candle_cake_state(true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let doused = click_use(&sender, &receiver, &mut world, at, 0.6, 1);
    assert_eq!(live_block(&mut world, at), Some(candle_cake_state(false)));
    assert!(
        sounds_sent(&doused).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.candle.extinguish")
                    .expect("a registered sound"),
            }),
        "{doused:?}"
    );
}

/// `ShovelItem.useOn` douses a lit campfire (`ShovelItem.java:51-69`).
#[test]
fn shovel_use_douses_a_lit_campfire() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:iron_shovel");
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(campfire_state(false)));
}

/// A waterlogged candle refuses flint (`CandleBlock.canLight`, `:166-168`).
#[test]
fn flint_use_does_not_light_a_waterlogged_candle() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let waterlogged = candle_state(false, true);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        waterlogged,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:flint_and_steel");
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(waterlogged));
}
