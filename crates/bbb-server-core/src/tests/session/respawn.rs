use super::*;
use bbb_protocol::item_types::{VANILLA_ITEM_CRAFTING_TABLE_ID, VANILLA_ITEM_OAK_PLANKS_ID};

fn click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    slot_num: i16,
    button_num: i8,
) -> TickResult {
    let menu = world.player_inventory_for_test(ConnectionId(1));
    let (container_id, state_id) = (menu.container_id(), menu.state_id());
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id,
            state_id,
            slot_num,
            button_num,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    )
}

#[test]
fn respawn_keeps_recipe_book_and_manual_crafting_in_both_grids() {
    for (keep_inventory, credits) in [(false, false), (true, false), (false, true)] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            datapacks: datapack_config().datapacks,
            ..operator_config(&[("alice", 1)])
        });
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        if keep_inventory {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "gamerule keep_inventory true",
            );
        }
        world.award_player_recipe_for_test(ConnectionId(1), "minecraft:crafting_table");
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
                book_type: RecipeBookType::Crafting,
                open: true,
                filtering: true,
            }),
        );
        for slot_num in 1..=4 {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                    slot_num,
                    item: ItemStackSummary {
                        item_id: Some(VANILLA_ITEM_OAK_PLANKS_ID),
                        count: 1,
                        component_patch: Default::default(),
                    },
                }),
            );
        }
        assert_eq!(
            world
                .player_crafting_result_for_test(ConnectionId(1))
                .item_id,
            Some(VANILLA_ITEM_CRAFTING_TABLE_ID)
        );
        if credits {
            world.set_won_game_for_test(ConnectionId(1));
        } else {
            chat_command(&sender, &receiver, &mut world, 1, "kill alice");
        }
        let respawned = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        );
        assert!(respawned.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Respawn(_),
                ..
            }
        )));
        let knows_recipe =
            world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:crafting_table");
        let settings = world.player_recipe_book_settings(ConnectionId(1))
            [RecipeBookType::Crafting.id() as usize];
        assert!(world
            .player_inventory_for_test(ConnectionId(1))
            .menu_items()
            .unwrap()[..5]
            .iter()
            .all(|item| item.item_id.is_none()));
        let teleport_id = respawned
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(position),
                    ..
                } => Some(position.id),
                _ => None,
            })
            .unwrap();
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::AcceptTeleportation { id: teleport_id },
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerLoaded,
        );

        let mut crafted_results = Vec::new();
        for table in [false, true] {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                    slot_num: 36,
                    item: ItemStackSummary {
                        item_id: Some(VANILLA_ITEM_OAK_PLANKS_ID),
                        count: 4,
                        component_patch: Default::default(),
                    },
                }),
            );
            if table {
                let at = BlockPos { x: 0, y: 64, z: 1 };
                let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                    bbb_protocol::block_types::VANILLA_BLOCK_CRAFTING_TABLE_ID,
                )
                .unwrap();
                assert!(world.set_block(
                    Java26BuiltinDimension::Overworld,
                    at,
                    state,
                    SET_BLOCK_UPDATE_ALL
                ));
                right_click(&sender, &receiver, &mut world, 1, at, 1);
                assert!(world
                    .player_inventory_for_test(ConnectionId(1))
                    .is_crafting_table_menu());
            }
            click(
                &sender,
                &receiver,
                &mut world,
                if table { 37 } else { 36 },
                0,
            );
            for slot in if table { [1, 2, 4, 5] } else { [1, 2, 3, 4] } {
                click(&sender, &receiver, &mut world, slot, 1);
            }
            click(&sender, &receiver, &mut world, 0, 0);
            let menu = world.player_inventory_for_test(ConnectionId(1));
            crafted_results.push((menu.carried().item_id(), menu.carried().count()));
            click(
                &sender,
                &receiver,
                &mut world,
                if table { 38 } else { 37 },
                0,
            );
        }
        assert_eq!(
            (knows_recipe, settings, crafted_results),
            (
                true,
                RecipeBookTypeSettings {
                    open: true,
                    filtering: true
                },
                vec![(Some(VANILLA_ITEM_CRAFTING_TABLE_ID), 1); 2]
            ),
            "keep_inventory={keep_inventory}, credits={credits}"
        );
    }
}

#[test]
fn death_respawn_fills_the_retained_max_health_base() {
    let mut restored = Vec::new();
    for max_health in [40.0, 10.0] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            game_type: GameType::Survival,
            ..operator_config(&[("alice", 1)])
        });
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("attribute alice minecraft:max_health base set {max_health}"),
        );
        assert_eq!(
            world.player_max_health_for_test(ConnectionId(1)),
            max_health
        );
        chat_command(&sender, &receiver, &mut world, 1, "kill alice");
        let respawned = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        );
        let reported = respawned.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetHealth(health),
                ..
            } => Some(health.health),
            _ => None,
        });
        restored.push((world.player_health_for_test(ConnectionId(1)), reported));
    }
    assert_eq!(restored, [(40.0, Some(40.0)), (10.0, Some(10.0))]);
}

#[test]
fn credits_respawn_restarts_absorption_without_losing_the_effect() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "effect give alice minecraft:absorption 600 1",
    );
    chat_command(&sender, &receiver, &mut world, 1, "damage alice 3");
    assert_eq!(world.player_absorption_for_test(ConnectionId(1)), 5.0);
    world.set_won_game_for_test(ConnectionId(1));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    world.tick(Vec::new());
    assert_eq!(
        (
            world.player_absorption_for_test(ConnectionId(1)),
            world
                .player_active_effect_for_test(ConnectionId(1), "minecraft:absorption")
                .map(|effect| effect.amplifier()),
        ),
        (8.0, Some(1))
    );
}
