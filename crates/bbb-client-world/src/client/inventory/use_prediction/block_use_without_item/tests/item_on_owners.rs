use super::*;

#[test]
fn fixed_client_branch_results_match_bed_note_light_fence_piston_and_dragon_egg() {
    for (name, expected) in [
        (
            "minecraft:red_bed",
            success(LocalInteractionSuccess::server_item_interaction()),
        ),
        (
            "minecraft:note_block",
            success(LocalInteractionSuccess::client_item_interaction()),
        ),
        (
            "minecraft:light",
            success(LocalInteractionSuccess::consume(false)),
        ),
        ("minecraft:oak_fence", pass()),
        ("minecraft:moving_piston", pass()),
        (
            "minecraft:dragon_egg",
            success(LocalInteractionSuccess::client_item_interaction()),
        ),
    ] {
        let mut world = block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        assert_eq!(use_main_hand(&mut world, 82), expected, "{name}");
        assert!(world.local_block_predictions().is_empty(), "{name}");
    }
}

#[test]
fn bell_result_follows_vanilla_attachment_face_and_hit_height() {
    let mut world = block_use_world(
        "minecraft:bell",
        [
            ("attachment", "floor"),
            ("facing", "north"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        world.predict_local_block_use_with_hit(
            POS,
            Direction::North,
            [0.5, 0.75, 0.0],
            false,
            InteractionHand::MainHand,
            false,
            91,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    for (face, cursor) in [
        (Direction::East, [1.0, 0.5, 0.5]),
        (Direction::North, [0.5, 0.9, 0.0]),
        (Direction::Up, [0.5, 1.0, 0.5]),
    ] {
        assert_eq!(
            world.predict_local_block_use_with_hit(
                POS,
                face,
                cursor,
                false,
                InteractionHand::MainHand,
                false,
                92,
            ),
            pass()
        );
    }

    set_test_block(
        &mut world,
        state_id(
            "minecraft:bell",
            [
                ("attachment", "single_wall"),
                ("facing", "north"),
                ("powered", "false"),
            ],
        ),
    );
    assert_eq!(
        world.predict_local_block_use_with_hit(
            POS,
            Direction::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            93,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
}

#[test]
fn chiseled_bookshelf_routes_front_hit_to_exact_occupied_slot() {
    let properties = [
        ("facing", "north"),
        ("slot_0_occupied", "false"),
        ("slot_1_occupied", "false"),
        ("slot_2_occupied", "false"),
        ("slot_3_occupied", "false"),
        ("slot_4_occupied", "false"),
        ("slot_5_occupied", "false"),
    ];
    let mut world = block_use_world_with_entity(
        "minecraft:chiseled_bookshelf",
        properties,
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
    );
    assert_eq!(
        world.predict_local_block_use_with_hit(
            POS,
            Direction::North,
            [0.9, 0.75, 0.0],
            false,
            InteractionHand::MainHand,
            false,
            101,
        ),
        success(LocalInteractionSuccess::consume(false)),
        "NORTH mirrors X, targeting empty slot 0"
    );
    assert_eq!(
        world.predict_local_block_use_with_hit(
            POS,
            Direction::East,
            [1.0, 0.75, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            102,
        ),
        pass(),
        "non-front faces expose no selectable slot"
    );

    set_test_block(
        &mut world,
        state_id(
            "minecraft:chiseled_bookshelf",
            [
                ("facing", "north"),
                ("slot_0_occupied", "true"),
                ("slot_1_occupied", "false"),
                ("slot_2_occupied", "false"),
                ("slot_3_occupied", "false"),
                ("slot_4_occupied", "false"),
                ("slot_5_occupied", "false"),
            ],
        ),
    );
    assert_eq!(
        world.predict_local_block_use_with_hit(
            POS,
            Direction::North,
            [0.9, 0.75, 0.0],
            false,
            InteractionHand::MainHand,
            false,
            103,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
}

#[test]
fn gamemaster_block_owners_require_matching_entity_permission_and_instabuild() {
    for (name, type_id) in [
        (
            "minecraft:command_block",
            VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
        ),
        ("minecraft:jigsaw", VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID),
        (
            "minecraft:structure_block",
            VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
        ),
        (
            "minecraft:test_block",
            VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
        ),
        (
            "minecraft:test_instance_block",
            VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        ),
    ] {
        let mut creative = block_use_world_first_state_with_entity(
            name,
            VANILLA_GAME_TYPE_CREATIVE_ID as i8,
            type_id,
        );
        assert_eq!(use_main_hand(&mut creative, 111), pass(), "{name}");
        assert!(creative.apply_entity_event(EntityEvent {
            entity_id: 7,
            event_id: 26,
        }));
        assert_eq!(
            use_main_hand(&mut creative, 112),
            success(LocalInteractionSuccess::client_item_interaction()),
            "{name}"
        );

        let mut survival = block_use_world_first_state_with_entity(
            name,
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            type_id,
        );
        assert!(survival.apply_entity_event(EntityEvent {
            entity_id: 7,
            event_id: 26,
        }));
        assert_eq!(
            use_main_hand(&mut survival, 113),
            pass(),
            "{name} permission without instabuild"
        );
    }
}

#[test]
fn entity_backed_pot_jukebox_and_sign_results_keep_server_mutations_authoritative() {
    for (name, type_id, expected) in [
        (
            "minecraft:decorated_pot",
            VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
            success(LocalInteractionSuccess::client_item_interaction()),
        ),
        (
            "minecraft:oak_sign",
            VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            success(LocalInteractionSuccess::consume(false)),
        ),
        (
            "minecraft:oak_hanging_sign",
            VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
            success(LocalInteractionSuccess::consume(false)),
        ),
    ] {
        let mut world = block_use_world_first_state_with_entity(
            name,
            VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            type_id,
        );
        assert_eq!(use_main_hand(&mut world, 121), expected, "{name}");

        let mut missing = block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        missing.remove_block_entity_record_at(POS);
        assert_eq!(use_main_hand(&mut missing, 122), pass(), "{name}");
    }

    let mut decorated = block_use_world_first_state_with_entity(
        "minecraft:decorated_pot",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
    );
    let block = decorated.block_view(POS).expect("loaded target");
    assert_eq!(
        decorated.predict_local_block_use_without_item(
            VanillaBlockUseWithoutItemOverride::DecoratedPotBlock,
            vanilla_block_type_for_resource_id("minecraft:decorated_pot").unwrap(),
            &block,
            LocalBlockUseContext {
                pos: POS,
                face: Direction::Up,
                cursor: [0.5, 1.0, 0.5],
                inside: false,
                sequence: 125,
                secondary_use_active: false,
            },
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );

    let mut jukebox = block_use_world_with_entity(
        "minecraft:jukebox",
        [("has_record", "true")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_JUKEBOX_ID,
    );
    assert_eq!(
        use_main_hand(&mut jukebox, 123),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_test_block(
        &mut jukebox,
        state_id("minecraft:jukebox", [("has_record", "false")]),
    );
    assert_eq!(use_main_hand(&mut jukebox, 124), pass());
}

#[test]
fn fixed_item_on_owners_route_exact_client_results() {
    let mut pumpkin =
        block_use_world_first_state("minecraft:pumpkin", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    set_main_hand(&mut pumpkin, VANILLA_ITEM_SHEARS_ID);
    assert_eq!(
        use_item_on(
            &mut pumpkin,
            "minecraft:pumpkin",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            126,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_main_hand(&mut pumpkin, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut pumpkin,
            "minecraft:pumpkin",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            127,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );

    let mut bush = block_use_world_first_state(
        "minecraft:sweet_berry_bush",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_test_block(
        &mut bush,
        state_id_with_property("minecraft:sweet_berry_bush", "age", "2"),
    );
    set_main_hand(&mut bush, VANILLA_ITEM_BONE_MEAL_ID);
    assert_eq!(
        use_item_on(
            &mut bush,
            "minecraft:sweet_berry_bush",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            128,
        ),
        pass()
    );
    set_test_block(
        &mut bush,
        state_id_with_property("minecraft:sweet_berry_bush", "age", "3"),
    );
    assert_eq!(
        use_item_on(
            &mut bush,
            "minecraft:sweet_berry_bush",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            129,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );

    for item_id in [VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FIRE_CHARGE_ID] {
        let mut tnt =
            block_use_world_first_state("minecraft:tnt", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        set_main_hand(&mut tnt, item_id);
        assert_eq!(
            use_item_on(
                &mut tnt,
                "minecraft:tnt",
                InteractionHand::MainHand,
                Direction::Up,
                [0.5, 1.0, 0.5],
                130,
            ),
            success(LocalInteractionSuccess::client_item_interaction())
        );
        assert!(tnt.local_block_predictions().is_empty());
    }

    let mut vault =
        block_use_world_first_state("minecraft:vault", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    set_test_block(
        &mut vault,
        state_id_with_property("minecraft:vault", "vault_state", "active"),
    );
    set_main_hand(&mut vault, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut vault,
            "minecraft:vault",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            131,
        ),
        success(LocalInteractionSuccess::server_item_interaction())
    );
}

#[test]
fn composter_item_on_uses_generated_membership_without_client_mutation() {
    let mut world = block_use_world(
        "minecraft:composter",
        [("level", "7")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        use_item_on(
            &mut world,
            "minecraft:composter",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            147,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(block_property(&world, "level").as_deref(), Some("7"));
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_APPLE_ID, 1))
    );
    assert!(world.local_block_predictions().is_empty());

    set_test_block(
        &mut world,
        state_id_with_property("minecraft:composter", "level", "8"),
    );
    assert_eq!(
        use_item_on(
            &mut world,
            "minecraft:composter",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            148,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
    set_test_block(
        &mut world,
        state_id_with_property("minecraft:composter", "level", "6"),
    );
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut world,
            "minecraft:composter",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            149,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
}

#[test]
fn beehive_bottle_use_forces_consumption_resets_honey_and_preserves_drop_edges() {
    let mut world = block_use_world(
        "minecraft:beehive",
        [("facing", "north"), ("honey_level", "4")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_main_hand(&mut world, VANILLA_ITEM_GLASS_BOTTLE_ID);
    assert_eq!(
        use_item_on(
            &mut world,
            "minecraft:beehive",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            150,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
    assert_eq!(block_property(&world, "honey_level").as_deref(), Some("4"));

    set_test_block(
        &mut world,
        state_id(
            "minecraft:beehive",
            [("facing", "north"), ("honey_level", "5")],
        ),
    );
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
    assert_eq!(
        use_item_on(
            &mut world,
            "minecraft:beehive",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            151,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction),
        "the shears branch starts only on ServerLevel"
    );

    set_main_hand(&mut world, VANILLA_ITEM_GLASS_BOTTLE_ID);
    assert_eq!(
        use_item_on(
            &mut world,
            "minecraft:beehive",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            152,
        ),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BeehiveBottleFill),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(block_property(&world, "honey_level").as_deref(), Some("0"));
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
    );
    assert_eq!(world.local_block_predictions()[0].sequence, 152);

    let fill_inventory = |world: &mut WorldStore, game_item_id| {
        for slot in 0..36 {
            assert!(world.apply_set_player_inventory(SetPlayerInventory {
                slot,
                item: item_stack(game_item_id, 64),
            }));
        }
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 2),
        }));
    };
    let honey_bottle_count = |world: &WorldStore| {
        world
            .inventory()
            .player_slots
            .iter()
            .filter(|slot| slot.item.item_id == Some(VANILLA_ITEM_HONEY_BOTTLE_ID))
            .map(|slot| slot.item.count)
            .sum::<i32>()
    };

    let mut full_survival = block_use_world(
        "minecraft:bee_nest",
        [("facing", "south"), ("honey_level", "5")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    fill_inventory(&mut full_survival, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut full_survival,
            "minecraft:bee_nest",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            153,
        ),
        success(LocalInteractionSuccess {
            extra_swing: Some(InteractionHand::MainHand),
            sound_action: Some(LocalInteractionSoundAction::BeehiveBottleFill),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(
        full_survival.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );
    assert_eq!(honey_bottle_count(&full_survival), 0);

    let mut full_creative = block_use_world(
        "minecraft:beehive",
        [("facing", "east"), ("honey_level", "5")],
        VANILLA_GAME_TYPE_CREATIVE_ID as i8,
    );
    fill_inventory(&mut full_creative, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut full_creative,
            "minecraft:beehive",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            154,
        ),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BeehiveBottleFill),
            ..LocalInteractionSuccess::client_item_interaction()
        }),
        "ItemStack.shrink bypasses instabuild while Inventory.add accepts overflow"
    );
    assert_eq!(
        full_creative.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );
    assert_eq!(honey_bottle_count(&full_creative), 0);
}

#[test]
fn redstone_ore_spawns_client_particles_before_exact_block_item_pass_result() {
    let mut placement = block_use_world(
        "minecraft:redstone_ore",
        [("lit", "false")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_main_hand(&mut placement, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_main_hand(&mut placement, 155),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPlaceFromProfile {
                pos: above(POS),
                block_type_id: VANILLA_BLOCK_STONE_ID,
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        }),
        "the ore PASS continues into the stone BlockItem placement"
    );
    assert_eq!(
        placement
            .probe_block(above(POS))
            .and_then(|block| block.block_name),
        Some("minecraft:stone".to_string())
    );
    assert_eq!(block_property(&placement, "lit").as_deref(), Some("false"));
    assert_eq!(
        placement.take_pending_local_interaction_particle_actions(),
        vec![LocalInteractionParticleAction::RedstoneOre { pos: POS }]
    );

    let mut obstructed = block_use_world(
        "minecraft:deepslate_redstone_ore",
        [("lit", "true")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_block_at(&mut obstructed, above(POS), state_id("minecraft:stone", []));
    set_main_hand(&mut obstructed, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut obstructed,
            "minecraft:deepslate_redstone_ore",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            156,
        ),
        success(LocalInteractionSuccess::client_item_interaction()),
        "BlockPlaceContext.canPlace is false when the adjacent block is solid"
    );
    assert_eq!(
        obstructed.take_pending_local_interaction_particle_actions(),
        vec![LocalInteractionParticleAction::RedstoneOre { pos: POS }]
    );

    set_main_hand(&mut obstructed, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        use_item_on(
            &mut obstructed,
            "minecraft:deepslate_redstone_ore",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            157,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        obstructed.take_pending_local_interaction_particle_actions(),
        vec![LocalInteractionParticleAction::RedstoneOre { pos: POS }]
    );
}

#[test]
fn weathering_copper_statue_routes_pose_wax_scrape_and_unaffected_axe_branches() {
    let apply_axes_tag = |world: &mut WorldStore| {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:axes".to_string(),
                    entries: vec![VANILLA_ITEM_IRON_AXE_ID],
                }],
            }],
        });
    };

    let mut pose = block_use_world_with_entity(
        "minecraft:weathered_copper_golem_statue",
        [
            ("facing", "north"),
            ("copper_golem_pose", "standing"),
            ("waterlogged", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
    );
    apply_axes_tag(&mut pose);
    assert_eq!(
        use_item_on(
            &mut pose,
            "minecraft:weathered_copper_golem_statue",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            158,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        block_property(&pose, "copper_golem_pose").as_deref(),
        Some("sitting")
    );

    set_main_hand_stack(&mut pose, item_stack(VANILLA_ITEM_HONEYCOMB_ID, 2));
    assert_eq!(
        use_main_hand(&mut pose, 159),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                pos: POS,
                sound_resource_id: "minecraft:item.honeycomb.wax_on",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 1.0_f32.to_bits(),
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        }),
        "the owner PASS reaches HoneycombItem.useOn"
    );
    assert_eq!(
        pose.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:waxed_weathered_copper_golem_statue".to_string())
    );
    assert_eq!(
        pose.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_HONEYCOMB_ID, 1))
    );

    let mut scrape = block_use_world_with_entity(
        "minecraft:exposed_copper_golem_statue",
        [
            ("facing", "east"),
            ("copper_golem_pose", "running"),
            ("waterlogged", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
    );
    apply_axes_tag(&mut scrape);
    set_main_hand(&mut scrape, VANILLA_ITEM_IRON_AXE_ID);
    assert_eq!(
        use_main_hand(&mut scrape, 160),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                pos: POS,
                sound_resource_id: "minecraft:item.axe.scrape",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 1.0_f32.to_bits(),
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        }),
        "the weathered owner PASS reaches AxeItem.useOn"
    );
    assert_eq!(
        scrape.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:copper_golem_statue".to_string())
    );
    assert_eq!(
        block_property(&scrape, "copper_golem_pose").as_deref(),
        Some("running")
    );

    let mut awaken = block_use_world_with_entity(
        "minecraft:copper_golem_statue",
        [
            ("facing", "south"),
            ("copper_golem_pose", "star"),
            ("waterlogged", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
    );
    apply_axes_tag(&mut awaken);
    set_main_hand(&mut awaken, VANILLA_ITEM_IRON_AXE_ID);
    assert_eq!(
        use_main_hand(&mut awaken, 161),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        awaken.probe_block(POS).and_then(|block| block.block_name),
        Some("minecraft:air".to_string())
    );
    assert!(
        awaken.probe_entities().is_empty(),
        "ClientLevel.addFreshEntity is a no-op"
    );
    assert_eq!(
        awaken.local_item_in_hand(InteractionHand::MainHand),
        Some(&item_stack(VANILLA_ITEM_IRON_AXE_ID, 1)),
        "hurtAndBreak(LivingEntity) only mutates inside its ServerLevel branch"
    );

    let mut waterlogged = block_use_world_with_entity(
        "minecraft:copper_golem_statue",
        [
            ("facing", "west"),
            ("copper_golem_pose", "sitting"),
            ("waterlogged", "true"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
    );
    apply_axes_tag(&mut waterlogged);
    set_main_hand(&mut waterlogged, VANILLA_ITEM_IRON_AXE_ID);
    assert!(matches!(
        use_main_hand(&mut waterlogged, 162),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        waterlogged
            .probe_block(POS)
            .and_then(|block| block.block_name),
        Some("minecraft:water".to_string()),
        "Level.removeBlock restores the waterlogged FluidState legacy block"
    );

    let mut missing_entity = block_use_world(
        "minecraft:oxidized_copper_golem_statue",
        [
            ("facing", "north"),
            ("copper_golem_pose", "standing"),
            ("waterlogged", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    missing_entity.remove_block_entity_record_at(POS);
    assert_eq!(missing_entity.block_entity_type_id_at(POS), None);
    set_main_hand(&mut missing_entity, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut missing_entity,
            "minecraft:oxidized_copper_golem_statue",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            163,
        ),
        pass()
    );
}

#[test]
fn cauldron_generated_item_routes_apply_exact_level_and_potion_predicates() {
    let mut empty =
        block_use_world_first_state("minecraft:cauldron", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    set_main_hand(&mut empty, VANILLA_ITEM_WATER_BUCKET_ID);
    assert_eq!(
        use_item_on(
            &mut empty,
            "minecraft:cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            150,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert!(empty.local_block_predictions().is_empty());

    let water_potion =
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water").unwrap();
    set_main_hand_stack(
        &mut empty,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(water_potion.id),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    assert_eq!(
        use_item_on(
            &mut empty,
            "minecraft:cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            151,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    let mundane =
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:mundane").unwrap();
    set_main_hand_stack(
        &mut empty,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(mundane.id),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    assert_eq!(
        use_item_on(
            &mut empty,
            "minecraft:cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            152,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );

    let mut water = block_use_world(
        "minecraft:water_cauldron",
        [("level", "2")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_main_hand(&mut water, VANILLA_ITEM_BUCKET_ID);
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            153,
        ),
        LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
            bbb_protocol::block_types::VanillaBlockUseItemOnOverride::AbstractCauldronBlock
        ),
        "tag dispatch precedes direct item dispatch"
    );
    apply_cauldron_remove_dye_tag(&mut water, Vec::new());
    set_main_hand_stack(
        &mut water,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(water_potion.id),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            154,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_main_hand(&mut water, VANILLA_ITEM_BUCKET_ID);
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            155,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
    set_test_block(
        &mut water,
        state_id_with_property("minecraft:water_cauldron", "level", "3"),
    );
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            156,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_main_hand_stack(
        &mut water,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(water_potion.id),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            157,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
    set_main_hand(&mut water, VANILLA_ITEM_GLASS_BOTTLE_ID);
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            158,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert!(water.local_block_predictions().is_empty());
}

#[test]
fn cauldron_tag_and_component_handlers_preserve_dispatch_priority() {
    let mut water = block_use_world(
        "minecraft:water_cauldron",
        [("level", "1")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    apply_cauldron_remove_dye_tag(&mut water, vec![VANILLA_ITEM_LEATHER_HELMET_ID]);
    set_main_hand_stack(
        &mut water,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_LEATHER_HELMET_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID],
                dyed_color: Some(0x12_34_56),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            157,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_main_hand(&mut water, VANILLA_ITEM_LEATHER_HELMET_ID);
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            158,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );

    set_main_hand_stack(
        &mut water,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_WHITE_BANNER_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID],
                banner_pattern_layers: vec![BannerPatternLayerSummary {
                    registry_id: Some(0),
                    asset_id: None,
                    translation_key: None,
                    color_id: VANILLA_DYE_COLOR_WHITE_ID,
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            159,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_main_hand(&mut water, VANILLA_ITEM_WHITE_BANNER_ID);
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            160,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
    set_main_hand(&mut water, VANILLA_ITEM_WHITE_SHULKER_BOX_ID);
    assert_eq!(
        use_item_on(
            &mut water,
            "minecraft:water_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            161,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
}

#[test]
fn cauldron_lava_and_powder_bucket_results_follow_water_above_and_level() {
    let mut empty =
        block_use_world_first_state("minecraft:cauldron", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    set_main_hand(&mut empty, VANILLA_ITEM_LAVA_BUCKET_ID);
    assert_eq!(
        use_item_on(
            &mut empty,
            "minecraft:cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            162,
        ),
        LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
            bbb_protocol::block_types::VanillaBlockUseItemOnOverride::AbstractCauldronBlock
        )
    );
    let water_holder = empty
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:water")
        .unwrap();
    let flowing_water_holder = empty
        .registry_holder_id_by_resource_id("minecraft:fluid", "minecraft:flowing_water")
        .unwrap();
    empty.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:fluid".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:water".to_string(),
                entries: vec![water_holder, flowing_water_holder],
            }],
        }],
    });
    assert_eq!(
        use_item_on(
            &mut empty,
            "minecraft:cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            163,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_block_at(
        &mut empty,
        above(POS),
        state_id("minecraft:water", [("level", "0")]),
    );
    assert_eq!(
        use_item_on(
            &mut empty,
            "minecraft:cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            164,
        ),
        success(LocalInteractionSuccess::consume(false))
    );

    let mut powder = block_use_world(
        "minecraft:powder_snow_cauldron",
        [("level", "2")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_main_hand(&mut powder, VANILLA_ITEM_BUCKET_ID);
    assert_eq!(
        use_item_on(
            &mut powder,
            "minecraft:powder_snow_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            165,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
    set_test_block(
        &mut powder,
        state_id_with_property("minecraft:powder_snow_cauldron", "level", "3"),
    );
    assert_eq!(
        use_item_on(
            &mut powder,
            "minecraft:powder_snow_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            166,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_main_hand(&mut powder, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID);
    assert_eq!(
        use_item_on(
            &mut powder,
            "minecraft:powder_snow_cauldron",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            167,
        ),
        LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
            bbb_protocol::block_types::VanillaBlockUseItemOnOverride::AbstractCauldronBlock
        ),
        "filled powder bucket still needs synchronized water fluid tag"
    );
}

#[test]
fn tag_gated_item_on_owners_use_synchronized_item_tags() {
    let tags = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:bookshelf_books".to_string(),
                    entries: vec![VANILLA_ITEM_BOOK_ID],
                },
                TagNetworkPayload {
                    tag: "minecraft:lectern_books".to_string(),
                    entries: vec![VANILLA_ITEM_BOOK_ID],
                },
                TagNetworkPayload {
                    tag: "minecraft:noteblock_top_instruments".to_string(),
                    entries: vec![VANILLA_ITEM_NOTE_BLOCK_ID],
                },
            ],
        }],
    };

    let mut shelf = block_use_world_with_entity(
        "minecraft:chiseled_bookshelf",
        [
            ("facing", "north"),
            ("slot_0_occupied", "false"),
            ("slot_1_occupied", "false"),
            ("slot_2_occupied", "false"),
            ("slot_3_occupied", "false"),
            ("slot_4_occupied", "false"),
            ("slot_5_occupied", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
    );
    shelf.apply_update_tags(tags.clone());
    set_main_hand(&mut shelf, VANILLA_ITEM_BOOK_ID);
    assert_eq!(
        use_item_on(
            &mut shelf,
            "minecraft:chiseled_bookshelf",
            InteractionHand::MainHand,
            Direction::North,
            [0.9, 0.75, 0.0],
            132,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    set_test_block(
        &mut shelf,
        state_id(
            "minecraft:chiseled_bookshelf",
            [
                ("facing", "north"),
                ("slot_0_occupied", "true"),
                ("slot_1_occupied", "false"),
                ("slot_2_occupied", "false"),
                ("slot_3_occupied", "false"),
                ("slot_4_occupied", "false"),
                ("slot_5_occupied", "false"),
            ],
        ),
    );
    assert_eq!(
        use_item_on(
            &mut shelf,
            "minecraft:chiseled_bookshelf",
            InteractionHand::MainHand,
            Direction::North,
            [0.9, 0.75, 0.0],
            133,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );

    let mut lectern =
        block_use_world_first_state("minecraft:lectern", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    lectern.apply_update_tags(tags.clone());
    set_test_block(
        &mut lectern,
        state_id_with_property("minecraft:lectern", "has_book", "false"),
    );
    set_main_hand(&mut lectern, VANILLA_ITEM_BOOK_ID);
    assert_eq!(
        use_item_on(
            &mut lectern,
            "minecraft:lectern",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            134,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );

    let mut note =
        block_use_world_first_state("minecraft:note_block", VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
    note.apply_update_tags(tags);
    set_main_hand(&mut note, VANILLA_ITEM_NOTE_BLOCK_ID);
    assert_eq!(
        use_item_on(
            &mut note,
            "minecraft:note_block",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            135,
        ),
        pass()
    );
    assert_eq!(
        use_item_on(
            &mut note,
            "minecraft:note_block",
            InteractionHand::MainHand,
            Direction::North,
            [0.5, 0.5, 0.0],
            136,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
    );
}

#[test]
fn entity_and_data_driven_item_on_owners_keep_server_mutations_authoritative() {
    let mut pot = block_use_world_first_state_with_entity(
        "minecraft:decorated_pot",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
    );
    set_main_hand(&mut pot, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut pot,
            "minecraft:decorated_pot",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            137,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );

    let mut shelf = block_use_world_with_entity(
        "minecraft:oak_shelf",
        [
            ("facing", "north"),
            ("powered", "false"),
            ("side_chain", "unconnected"),
            ("waterlogged", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID,
    );
    set_main_hand(&mut shelf, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        use_item_on(
            &mut shelf,
            "minecraft:oak_shelf",
            InteractionHand::MainHand,
            Direction::North,
            [0.5, 0.5, 0.0],
            138,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        use_item_on(
            &mut shelf,
            "minecraft:oak_shelf",
            InteractionHand::OffHand,
            Direction::North,
            [0.5, 0.5, 0.0],
            139,
        ),
        pass()
    );

    let mut campfire = block_use_world_first_state_with_entity(
        "minecraft:campfire",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
    );
    campfire.apply_update_recipes(UpdateRecipes {
        property_sets: vec![RecipePropertySetSummary {
            key: "minecraft:campfire_input".to_string(),
            item_ids: vec![VANILLA_ITEM_OAK_LOG_ID],
        }],
        stonecutter_recipes: Vec::new(),
    });
    set_main_hand(&mut campfire, VANILLA_ITEM_OAK_LOG_ID);
    assert_eq!(
        use_item_on(
            &mut campfire,
            "minecraft:campfire",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            140,
        ),
        success(LocalInteractionSuccess::consume(false))
    );

    let mut jukebox = block_use_world(
        "minecraft:jukebox",
        [("has_record", "false")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    jukebox.set_default_item_component_codec_values(BTreeMap::from([(
        VANILLA_ITEM_MUSIC_DISC_13_ID,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            serde_json::json!("minecraft:13"),
        )]),
    )]));
    set_main_hand(&mut jukebox, VANILLA_ITEM_MUSIC_DISC_13_ID);
    assert_eq!(
        use_item_on(
            &mut jukebox,
            "minecraft:jukebox",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            141,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );

    let mut sign = block_use_world_first_state_with_entity(
        "minecraft:oak_sign",
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
    );
    set_main_hand(&mut sign, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        use_item_on(
            &mut sign,
            "minecraft:oak_sign",
            InteractionHand::MainHand,
            Direction::North,
            [0.5, 0.5, 0.0],
            142,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
}

#[test]
fn stateful_item_on_owners_predict_vanilla_state_inventory_and_effect_actions() {
    let mut candle = block_use_world(
        "minecraft:candle",
        [("candles", "4"), ("lit", "true"), ("waterlogged", "false")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_item_on(
            &mut candle,
            "minecraft:candle",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            143,
        ),
        success(LocalInteractionSuccess {
            sound_action: None,
            particle_action: Some(LocalInteractionParticleAction::CandleExtinguish {
                pos: POS,
                candles: 4,
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(block_property(&candle, "lit").as_deref(), Some("false"));

    let mut anchor = block_use_world(
        "minecraft:respawn_anchor",
        [("charges", "0")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_main_hand(&mut anchor, VANILLA_ITEM_GLOWSTONE_ID);
    assert_eq!(
        use_item_on(
            &mut anchor,
            "minecraft:respawn_anchor",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            144,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(block_property(&anchor, "charges").as_deref(), Some("1"));
    assert!(anchor
        .local_item_in_hand(InteractionHand::MainHand)
        .is_none());

    let mut statue = block_use_world(
        "minecraft:waxed_copper_golem_statue",
        [
            ("facing", "north"),
            ("copper_golem_pose", "star"),
            ("waterlogged", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_item_on(
            &mut statue,
            "minecraft:waxed_copper_golem_statue",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            145,
        ),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        block_property(&statue, "copper_golem_pose").as_deref(),
        Some("standing")
    );

    statue.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:axes".to_string(),
                entries: vec![VANILLA_ITEM_IRON_AXE_ID],
            }],
        }],
    });
    set_main_hand(&mut statue, VANILLA_ITEM_IRON_AXE_ID);
    assert_eq!(
        use_item_on(
            &mut statue,
            "minecraft:waxed_copper_golem_statue",
            InteractionHand::MainHand,
            Direction::Up,
            [0.5, 1.0, 0.5],
            146,
        ),
        pass()
    );
    assert_eq!(
        block_property(&statue, "copper_golem_pose").as_deref(),
        Some("standing")
    );
}

#[test]
fn property_gated_server_owned_actions_return_exact_client_results() {
    for (name, property, active, inactive, active_result, inactive_result) in [
        (
            "minecraft:cave_vines_plant",
            "berries",
            "true",
            "false",
            success(LocalInteractionSuccess::client_item_interaction()),
            pass(),
        ),
        (
            "minecraft:composter",
            "level",
            "8",
            "7",
            success(LocalInteractionSuccess::client_item_interaction()),
            pass(),
        ),
        (
            "minecraft:lectern",
            "has_book",
            "true",
            "false",
            success(LocalInteractionSuccess::client_item_interaction()),
            pass(),
        ),
        (
            "minecraft:respawn_anchor",
            "charges",
            "1",
            "0",
            success(LocalInteractionSuccess::consume(false)),
            pass(),
        ),
        (
            "minecraft:sweet_berry_bush",
            "age",
            "2",
            "1",
            success(LocalInteractionSuccess::client_item_interaction()),
            pass(),
        ),
    ] {
        let mut world = block_use_world_first_state(name, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        set_test_block(&mut world, state_id_with_property(name, property, active));
        assert_eq!(
            use_main_hand(&mut world, 83),
            active_result,
            "{name} active"
        );

        set_test_block(&mut world, state_id_with_property(name, property, inactive));
        assert_eq!(
            use_main_hand(&mut world, 84),
            inactive_result,
            "{name} inactive"
        );
        assert!(world.local_block_predictions().is_empty(), "{name}");
    }
}
