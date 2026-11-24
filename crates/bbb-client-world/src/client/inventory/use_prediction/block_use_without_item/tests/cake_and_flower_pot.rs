use super::*;

#[test]
fn cake_eating_updates_food_saturation_and_the_exact_predicted_state() {
    let mut cake = block_use_world(
        "minecraft:cake",
        [("bites", "0")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    cake.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 18,
        saturation: 2.0,
    });
    assert_eq!(
        use_main_hand(&mut cake, 131),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(block_property(&cake, "bites").as_deref(), Some("1"));
    assert_eq!(
        cake.local_player().health,
        Some(LocalPlayerHealthState {
            health: 20.0,
            food: 20,
            saturation: 2.4,
        })
    );
    assert_eq!(cake.local_block_predictions()[0].sequence, 131);

    let mut last_slice = block_use_world(
        "minecraft:cake",
        [("bites", "6")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    last_slice.apply_player_health(PlayerHealth {
        health: 19.0,
        food: 17,
        saturation: 1.0,
    });
    assert_eq!(
        use_main_hand(&mut last_slice, 132),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        last_slice
            .probe_block(POS)
            .and_then(|block| block.block_name),
        Some("minecraft:air".to_string())
    );
    assert_eq!(last_slice.local_player().health.unwrap().food, 19);
}

#[test]
fn full_food_cake_consumes_empty_hand_but_candle_cake_passes() {
    let mut cake = block_use_world(
        "minecraft:cake",
        [("bites", "0")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    cake.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 20,
        saturation: 5.0,
    });
    assert_eq!(
        use_main_hand(&mut cake, 133),
        success(LocalInteractionSuccess::consume(false))
    );
    assert_eq!(block_property(&cake, "bites").as_deref(), Some("0"));
    assert!(cake.local_block_predictions().is_empty());

    let mut candle_cake = block_use_world(
        "minecraft:candle_cake",
        [("lit", "false")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    candle_cake.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 20,
        saturation: 5.0,
    });
    assert_eq!(use_main_hand(&mut candle_cake, 134), pass());
    assert_eq!(
        candle_cake
            .probe_block(POS)
            .and_then(|block| block.block_name),
        Some("minecraft:candle_cake".to_string())
    );
}

#[test]
fn candle_cake_eating_replaces_it_with_one_bite_of_cake() {
    let mut world = block_use_world(
        "minecraft:white_candle_cake",
        [("lit", "false")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 16,
        saturation: 1.0,
    });
    assert_eq!(
        use_main_hand(&mut world, 135),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        world.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:cake".to_string())
    );
    assert_eq!(block_property(&world, "bites").as_deref(), Some("1"));
    assert_eq!(world.local_player().health.unwrap().food, 18);
}

#[test]
fn lit_candle_cake_hit_extinguishes_before_the_empty_hand_eat_path() {
    let mut world = block_use_world(
        "minecraft:red_candle_cake",
        [("lit", "true")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 16,
        saturation: 1.0,
    });
    assert_eq!(
        world.predict_local_block_use_with_hit(
            POS,
            Direction::Up,
            [0.5, 0.75, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            136,
        ),
        success(LocalInteractionSuccess {
            sound_action: None,
            particle_action: Some(LocalInteractionParticleAction::CandleCakeExtinguish {
                pos: POS,
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(block_property(&world, "lit").as_deref(), Some("false"));
    assert_eq!(world.local_player().health.unwrap().food, 16);
    assert_eq!(world.local_block_predictions()[0].sequence, 136);
}

#[test]
fn candle_item_on_whole_cake_predicts_candle_cake_and_consumes_one() {
    let mut world = block_use_world(
        "minecraft:cake",
        [("bites", "0")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert!(world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_CANDLE_ID, 2),
    }));
    assert_eq!(
        use_main_hand(&mut world, 137),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        world.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:candle_cake".to_string())
    );
    assert_eq!(block_property(&world, "lit").as_deref(), Some("false"));
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_CANDLE_ID, 1))
    );
}

#[test]
fn flower_pot_empty_and_potted_branches_update_inventory_and_block_state() {
    let mut empty =
        block_use_world_first_state("minecraft:flower_pot", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    assert_eq!(
        use_main_hand(&mut empty, 141),
        success(LocalInteractionSuccess::consume(false))
    );
    assert!(empty.local_block_predictions().is_empty());

    let mut potted = block_use_world_first_state(
        "minecraft:potted_poppy",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_main_hand(&mut potted, 142),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        potted.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:flower_pot".to_string())
    );
    assert_eq!(
        potted.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_POPPY_ID, 1))
    );

    let mut plant =
        block_use_world_first_state("minecraft:flower_pot", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    assert!(plant.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: item_stack(VANILLA_ITEM_POPPY_ID, 2),
    }));
    assert_eq!(
        use_main_hand(&mut plant, 144),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        plant.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:potted_poppy".to_string())
    );
    assert_eq!(
        plant.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_POPPY_ID, 1))
    );

    assert_eq!(
        use_main_hand(&mut plant, 145),
        success(LocalInteractionSuccess::consume(false)),
        "a pottable item against an occupied pot consumes without replacing it"
    );
    assert_eq!(
        plant.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:potted_poppy".to_string())
    );
}

#[test]
fn potted_flower_full_inventory_requests_vanilla_drop_swing() {
    let mut world = block_use_world_first_state(
        "minecraft:potted_poppy",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    for slot in (0..36).chain(std::iter::once(40)) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot,
            item: item_stack(VANILLA_ITEM_STONE_ID, 64),
        }));
    }
    assert_eq!(
        use_main_hand(&mut world, 143),
        success(LocalInteractionSuccess {
            extra_swing: Some(InteractionHand::MainHand),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(
        world.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:flower_pot".to_string())
    );
}

#[test]
fn every_generated_potted_flower_owner_maps_back_to_its_block_item() {
    let mut owner_count = 0;
    for block_type in VANILLA_BLOCK_TYPES.iter().filter(|block_type| {
        block_type.use_without_item_override
            == Some(VanillaBlockUseWithoutItemOverride::FlowerPotBlock)
    }) {
        owner_count += 1;
        if block_type.resource_id == "minecraft:flower_pot" {
            assert_eq!(potted_flower_item_id(block_type), None);
        } else {
            assert!(
                potted_flower_item_id(block_type).is_some(),
                "{}",
                block_type.resource_id
            );
        }
    }
    assert!(owner_count > 1);
    assert_eq!(
        potted_flower_item_id(&VANILLA_BLOCK_TYPES[VANILLA_BLOCK_POTTED_POPPY_ID as usize]),
        Some(VANILLA_ITEM_POPPY_ID)
    );
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
