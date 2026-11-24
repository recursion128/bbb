//! `Consumable.startConsuming` / `onConsume` (`Consumable.java:62-93`) for a
//! session: the use duration, the per-tick sounds, the listeners and effects,
//! the use remainder and the interruptions.

use super::*;

/// Bread is `Foods.BREAD` (`food(5, 0.6F)`) under the default 1.6-second
/// consumable: 32 ticks of use, of which the one the packet lands in is the
/// first.
#[test]
fn eating_bread_runs_thirty_two_ticks_then_feeds_and_consumes_one() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:bread", 2));
    let alice = fixture.alice;

    let started = fixture.use_item(InteractionHand::MainHand);
    // `ServerPlayerGameMode.useItem` (`:340-342`): a use in progress is not
    // followed by `sendAllDataToRemote`.
    assert!(!packets_to(&started, ALICE)
        .any(|packet| matches!(packet, ClientboundPlayPacket::ContainerSetContent(_))));
    assert_eq!(living_flags(&started, BOB, alice), [1]);
    assert_eq!(living_flags(&started, ALICE, alice), [1]);
    let (using, hand, remaining, use_item) = fixture.use_state();
    assert!(using);
    assert_eq!(hand, InteractionHand::MainHand);
    assert_eq!(remaining, 31);
    assert_eq!(use_item.item_id, Some(item_id("minecraft:bread")));

    let mut eat_sounds_heard_by_bob = 0;
    for tick in 2..32 {
        let idle = fixture.idle();
        assert!(entity_events(&idle, ALICE, alice).is_empty(), "tick {tick}");
        assert!(sounds(&idle, ALICE).is_empty(), "tick {tick}");
        eat_sounds_heard_by_bob += sounds(&idle, BOB)
            .iter()
            .filter(|sound| **sound == "minecraft:entity.generic.eat")
            .count();
    }
    // `shouldEmitParticlesAndSounds` (`Consumable.java:119-124`) once the
    // first seven ticks are spent, on every fourth remaining tick: 24, 20,
    // 16, 12, 8 and 4.
    assert_eq!(eat_sounds_heard_by_bob, 6);
    assert_eq!(fixture.use_state().2, 1);
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 2);

    let finished = fixture.idle();
    // `ServerPlayer.completeUsingItem` (`ServerPlayer.java:1567-1572`).
    assert_eq!(entity_events(&finished, ALICE, alice), [9]);
    assert!(entity_events(&finished, BOB, alice).is_empty());
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 1);
    assert_eq!(
        fixture
            .world
            .player_food_levels_for_test(ConnectionId(ALICE)),
        (15, 6.0)
    );
    assert!(!fixture.use_state().0);
    assert_eq!(fixture.item_used("minecraft:bread"), 1);
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "minecraft:husbandry/root"));
    // `onConsume`'s eat sound skips the eater; the food listener's two do not.
    assert_eq!(
        sounds(&finished, BOB),
        [
            "minecraft:entity.generic.eat",
            "minecraft:entity.generic.eat",
            "minecraft:entity.player.burp",
        ]
    );
    assert_eq!(
        sounds(&finished, ALICE),
        [
            "minecraft:entity.generic.eat",
            "minecraft:entity.player.burp"
        ]
    );

    let published = fixture.idle();
    assert_eq!(living_flags(&published, BOB, alice), [0]);
}

/// `Player.canEat` (`Player.java:1581-1583`): a full survival player's use
/// fails without starting.
#[test]
fn a_full_player_cannot_start_eating() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 20, 5.0);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:bread", 1));
    let refused = fixture.use_item(InteractionHand::MainHand);
    assert!(!fixture.use_state().0);
    assert!(living_flags(&refused, BOB, fixture.alice).is_empty());
    // `FAIL` with a positive use duration and no use running returns before
    // the menu is republished (`ServerPlayerGameMode.java:330-332`).
    assert!(!packets_to(&refused, ALICE)
        .any(|packet| matches!(packet, ClientboundPlayPacket::ContainerSetContent(_))));

    // `can_always_eat` foods pass the same gate.
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:golden_apple", 1));
    fixture.use_item(InteractionHand::MainHand);
    assert!(fixture.use_state().0);
}

/// Changing the selected slot (`ServerGamePacketListenerImpl.java:1473-1475`)
/// and `RELEASE_USE_ITEM` (`:1302-1304`) both end the use before it finishes.
#[test]
fn switching_slots_or_releasing_stops_eating_without_consuming() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:bread", 2));
    let alice = fixture.alice;

    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..10 {
        fixture.idle();
    }
    let switched = fixture.play(ALICE, PlayServerbound::SetCarriedItem { slot: 1 });
    assert!(!fixture.use_state().0);
    assert_eq!(living_flags(&switched, BOB, alice), [0]);

    fixture.play(ALICE, PlayServerbound::SetCarriedItem { slot: 0 });
    fixture.use_item(InteractionHand::MainHand);
    assert!(fixture.use_state().0);
    let released = fixture.action(PlayerActionKind::ReleaseUseItem);
    assert!(!fixture.use_state().0);
    assert_eq!(living_flags(&released, BOB, alice), [0]);

    for _ in 0..40 {
        fixture.idle();
    }
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 2);
    assert_eq!(
        fixture
            .world
            .player_food_levels_for_test(ConnectionId(ALICE))
            .0,
        10
    );
    assert_eq!(fixture.item_used("minecraft:bread"), 0);
}

/// An offhand use raises both bits, and moving the stack out of the hand
/// ends it at the next `updatingUsingItem` (`LivingEntity.java:3361-3370`).
#[test]
fn an_offhand_use_publishes_the_hand_bit_and_ends_when_the_hand_changes() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    fixture.hold(OFFHAND_MENU_SLOT, stack("minecraft:apple", 1));
    let alice = fixture.alice;

    let started = fixture.use_item(InteractionHand::OffHand);
    assert_eq!(living_flags(&started, BOB, alice), [3]);

    fixture.hold(OFFHAND_MENU_SLOT, stack("minecraft:stone", 1));
    fixture.idle();
    assert!(!fixture.use_state().0);
    assert_eq!(fixture.slot(OFFHAND_SLOT), stack("minecraft:stone", 1));
    // `stopUsingItem` clears only the using bit (`LivingEntity.java:3554`).
    let published = fixture.idle();
    assert_eq!(living_flags(&published, BOB, alice), [2]);
}

/// A potion is `Consumables.DEFAULT_DRINK` over `PotionContents.onConsume`
/// (`PotionContents.java:235-237`), and `usingConvertsTo(GLASS_BOTTLE)` hands
/// back the bottle once the last potion is drunk.
#[test]
fn drinking_a_potion_applies_its_effect_and_leaves_the_bottle() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture.hold(HOTBAR_MENU_SLOT, potion("minecraft:swiftness"));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..30 {
        fixture.idle();
    }
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .is_none());
    let finished = fixture.idle();
    let speed = fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .expect("the potion's effect");
    assert_eq!((speed.duration(), speed.amplifier()), (3600, 0));
    assert!(packets_to(&finished, ALICE)
        .skip_while(|packet| !matches!(
            packet,
            ClientboundPlayPacket::EntityEvent(event)
                if event.entity_id == fixture.alice && event.event_id == 9
        ))
        .any(|packet| matches!(packet, ClientboundPlayPacket::UpdateMobEffect(_))));
    assert_eq!(
        fixture.slot(HOTBAR_SLOT),
        stack("minecraft:glass_bottle", 1)
    );
    assert!(sounds(&finished, BOB).contains(&"minecraft:entity.generic.drink"));
    assert_eq!(fixture.item_used("minecraft:potion"), 1);
}

/// `ClearAllStatusEffectsConsumeEffect` (`ClearAllStatusEffectsConsumeEffect.java:21-23`)
/// through the milk bucket, whose remainder is the empty bucket; in creative
/// the bucket stays full (`UseRemainder.java:21-23`).
#[test]
fn milk_clears_every_effect_and_returns_the_bucket_outside_creative() {
    for (game_type, left) in [
        (GameType::Survival, "minecraft:bucket"),
        (GameType::Creative, "minecraft:milk_bucket"),
    ] {
        let mut fixture = Fixture::new(game_type);
        fixture.hold(HOTBAR_MENU_SLOT, potion("minecraft:swiftness"));
        fixture.use_item(InteractionHand::MainHand);
        for _ in 0..31 {
            fixture.idle();
        }
        assert!(fixture
            .world
            .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
            .is_some());
        fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:milk_bucket", 1));
        fixture.use_item(InteractionHand::MainHand);
        let mut removed = false;
        for _ in 0..31 {
            let tick = fixture.idle();
            removed |= packets_to(&tick, ALICE)
                .any(|packet| matches!(packet, ClientboundPlayPacket::RemoveMobEffect(_)));
        }
        assert!(removed, "{game_type:?}");
        assert!(fixture
            .world
            .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
            .is_none());
        assert_eq!(fixture.slot(HOTBAR_SLOT), stack(left, 1), "{game_type:?}");
    }
}

/// `Consumable.onConsume` fires `consume_item` (`Consumable.java:79-82`)
/// before the stack's listeners and `onConsumeEffects` run, so the criterion
/// still sees the effect the milk is about to clear.
#[test]
fn consume_item_sees_the_player_before_the_consume_effects() {
    let mut config = datapack_config();
    let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
    startup
        .advancements
        .advancements
        .push(bbb_server_datapack::advancement::AdvancementElement {
            id: "test:milk_while_fast".into(),
            source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(&serde_json::json!({
                "criteria":{"drank":{"trigger":"minecraft:consume_item","conditions":{
                    "item":{"items":"minecraft:milk_bucket"},
                    "player":{"effects":{"minecraft:speed":{}}}
                }}}
            }))
            .unwrap(),
        });
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.hold(HOTBAR_MENU_SLOT, potion("minecraft:swiftness"));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..31 {
        fixture.idle();
    }
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:milk_bucket", 1));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..31 {
        fixture.idle();
    }
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .is_none());
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:milk_while_fast"));
}

/// A stack of two honey bottles: the first drink leaves one bottle in hand
/// and puts the glass bottle elsewhere in the inventory
/// (`ServerPlayer.handleExtraItemsCreatedOnUse`, `ServerPlayer.java:2078-2082`),
/// and the honey's `RemoveStatusEffectsConsumeEffect` cures poison.
#[test]
fn a_partial_stack_puts_its_remainder_in_the_inventory() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    let mut events = Vec::new();
    fixture.world.add_player_mob_effect(
        ConnectionId(ALICE),
        crate::entity::living::mob_effects::ActiveMobEffect::from_saved_with_hidden(
            "minecraft:poison",
            400,
            0,
            false,
            true,
            true,
            None,
        )
        .expect("poison"),
        &mut events,
    );
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:honey_bottle", 2));
    fixture.use_item(InteractionHand::MainHand);
    // `Consumables.HONEY_BOTTLE` is two seconds long.
    for _ in 0..38 {
        fixture.idle();
    }
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:poison")
        .is_some());
    fixture.idle();
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:poison")
        .is_none());
    assert_eq!(
        fixture.slot(HOTBAR_SLOT),
        stack("minecraft:honey_bottle", 1)
    );
    assert_eq!(fixture.slot(1), stack("minecraft:glass_bottle", 1));
}

/// `usingConvertsTo(BOWL)` on a one-stack stew, and the suspicious stew's own
/// `SuspiciousStewEffects.onConsume` (`SuspiciousStewEffects.java:39-43`) on
/// its built-in empty effect list.
#[test]
fn stew_leaves_its_bowl() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:mushroom_stew", 1));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..31 {
        fixture.idle();
    }
    assert_eq!(fixture.slot(HOTBAR_SLOT), stack("minecraft:bowl", 1));
    assert_eq!(
        fixture
            .world
            .player_food_levels_for_test(ConnectionId(ALICE))
            .0,
        16
    );
}

/// `TeleportRandomlyConsumeEffect` (`TeleportRandomlyConsumeEffect.java:41-79`)
/// moves the eater with a position packet, and the chorus fruit's
/// `use_cooldown` (`Items.java:1810-1811`) starts once it is eaten.
#[test]
fn chorus_fruit_teleports_and_starts_its_cooldown() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    let fruit = stack("minecraft:chorus_fruit", 2);
    fixture.hold(HOTBAR_MENU_SLOT, fruit.clone());
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..30 {
        fixture.idle();
    }
    let finished = fixture.idle();
    let moves = packets_to(&finished, ALICE)
        .filter(|packet| matches!(packet, ClientboundPlayPacket::PlayerPosition(_)))
        .count();
    assert!(moves >= 1, "{:#?}", finished.events);
    assert!(packets_to(&finished, ALICE).any(|packet| matches!(
        packet,
        ClientboundPlayPacket::Cooldown(cooldown) if cooldown.duration == 20
    )));
    assert!(fixture
        .world
        .player_is_on_cooldown_for_test(ConnectionId(ALICE), &fruit));
    // The cooldown gate (`ServerPlayerGameMode.java:309-311`); the one flag
    // write this tick publishes is the finished use's.
    let refused = fixture.use_item(InteractionHand::MainHand);
    assert!(!fixture.use_state().0);
    assert_eq!(living_flags(&refused, BOB, fixture.alice), [0]);
}

/// `FoodData.addAdditionalSaveData` and the active effects
/// `LivingEntity.addAdditionalSaveData` writes (`LivingEntity.java:732-734`)
/// carry what a golden apple left across a save and a restart.
#[test]
fn a_golden_apples_food_and_effects_survive_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-using-item-golden-apple-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-golden-apple");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = || ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        ..datapack_config()
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config(), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    world.set_player_food_for_test(ConnectionId(ALICE), 10, 0.0);
    world.stock_menu_slot_for_test(
        ConnectionId(ALICE),
        HOTBAR_MENU_SLOT,
        stack("minecraft:golden_apple", 1),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        ALICE,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    for _ in 1..32 {
        world.tick(receiver.drain());
    }
    let eaten = world.player_food_levels_for_test(ConnectionId(ALICE));
    assert_eq!(eaten, (14, 4.0 * 1.2 * 2.0));
    let effect = |world: &ServerWorld, resource_id| {
        world
            .player_active_effect_for_test(ConnectionId(ALICE), resource_id)
            .map(|effect| (effect.amplifier(), effect.duration()))
    };
    let regeneration = effect(&world, "minecraft:regeneration").expect("the apple's regeneration");
    let absorption = effect(&world, "minecraft:absorption").expect("the apple's absorption");
    assert_eq!(regeneration.0, 1);
    assert_eq!(absorption.0, 0);

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config(), &reloaded).unwrap();
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    let (food, saturation) = world.player_food_levels_for_test(ConnectionId(ALICE));
    assert_eq!(food, 14);
    // The two join ticks may already have spent saturation on healing.
    assert!(saturation <= eaten.1 && saturation > 0.0, "{saturation}");
    assert_eq!(
        effect(&world, "minecraft:regeneration").map(|(amplifier, _)| amplifier),
        Some(1)
    );
    assert_eq!(
        effect(&world, "minecraft:absorption").map(|(amplifier, _)| amplifier),
        Some(0)
    );
    let _ = std::fs::remove_dir_all(&root);
}
