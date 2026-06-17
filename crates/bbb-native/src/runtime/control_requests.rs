use bbb_control::{
    CodeOfConductControlRequest, ContainerClickControlRequest, ContainerInputControl,
    HashedComponentPatchControl, HashedStackControl, NetControlRequest, NetCounters,
    RecipeBookTypeControl, SharedSnapshot,
};
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, ContainerClick, ContainerInput, HashedComponentPatch,
    HashedItemStack, HashedStack, RecipeBookType, RenameItem,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;

use crate::{
    code_of_conduct::CodeOfConductAcceptance,
    input::{
        queue_chat_command, queue_command_suggestion_request, queue_container_button_click_command,
        queue_container_click_command, queue_container_close_request_command,
        queue_container_slot_state_changed_command, queue_place_recipe_command,
        queue_player_abilities_command, queue_recipe_book_change_settings_command,
        queue_recipe_book_seen_recipe_command, queue_rename_item_command,
        queue_select_trade_command, queue_sign_update_command, select_bundle_item,
        select_hotbar_slot,
    },
};

fn protocol_container_click(click: ContainerClickControlRequest) -> ContainerClick {
    ContainerClick {
        container_id: click.container_id,
        state_id: click.state_id,
        slot_num: click.slot_num,
        button_num: click.button_num,
        input: protocol_container_input(click.input),
        changed_slots: click
            .changed_slots
            .into_iter()
            .map(|changed| (changed.slot, protocol_hashed_stack(changed.stack)))
            .collect(),
        carried_item: protocol_hashed_stack(click.carried_item),
    }
}

fn protocol_container_input(input: ContainerInputControl) -> ContainerInput {
    match input {
        ContainerInputControl::Pickup => ContainerInput::Pickup,
        ContainerInputControl::QuickMove => ContainerInput::QuickMove,
        ContainerInputControl::Swap => ContainerInput::Swap,
        ContainerInputControl::Clone => ContainerInput::Clone,
        ContainerInputControl::Throw => ContainerInput::Throw,
        ContainerInputControl::QuickCraft => ContainerInput::QuickCraft,
        ContainerInputControl::PickupAll => ContainerInput::PickupAll,
    }
}

fn protocol_hashed_stack(stack: HashedStackControl) -> HashedStack {
    match stack {
        HashedStackControl::Empty => HashedStack::Empty,
        HashedStackControl::Item {
            item_id,
            count,
            components,
        } => HashedStack::Item(HashedItemStack {
            item_id,
            count,
            components: protocol_hashed_components(components),
        }),
    }
}

fn protocol_hashed_components(components: HashedComponentPatchControl) -> HashedComponentPatch {
    HashedComponentPatch {
        added_components: components.added_components,
        removed_components: components.removed_components,
    }
}

fn protocol_recipe_book_type(book_type: RecipeBookTypeControl) -> RecipeBookType {
    match book_type {
        RecipeBookTypeControl::Crafting => RecipeBookType::Crafting,
        RecipeBookTypeControl::Furnace => RecipeBookType::Furnace,
        RecipeBookTypeControl::BlastFurnace => RecipeBookType::BlastFurnace,
        RecipeBookTypeControl::Smoker => RecipeBookType::Smoker,
    }
}

pub(crate) fn pump_control_net_requests(
    snapshot: &SharedSnapshot,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    code_of_conduct: Option<&mut CodeOfConductAcceptance>,
) {
    let (requests, net_requests) = snapshot
        .write()
        .map(|mut guard| {
            (
                std::mem::take(&mut guard.code_of_conduct_requests),
                std::mem::take(&mut guard.net_requests),
            )
        })
        .unwrap_or_default();

    for request in net_requests {
        match request {
            NetControlRequest::SetHeldSlot { slot } => {
                select_hotbar_slot(counters, world, net_commands, slot);
            }
            NetControlRequest::SetFlying { flying } => {
                queue_player_abilities_command(counters, world, net_commands, flying);
            }
            NetControlRequest::PlaceRecipe {
                container_id,
                recipe_index,
                use_max_items,
            } => {
                queue_place_recipe_command(
                    counters,
                    net_commands,
                    bbb_protocol::packets::PlaceRecipeCommand {
                        container_id,
                        recipe_index,
                        use_max_items,
                    },
                );
            }
            NetControlRequest::ChangeRecipeBookSettings {
                book_type,
                open,
                filtering,
            } => {
                queue_recipe_book_change_settings_command(
                    counters,
                    net_commands,
                    bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                        book_type: protocol_recipe_book_type(book_type),
                        open,
                        filtering,
                    },
                );
            }
            NetControlRequest::MarkRecipeSeen { recipe_index } => {
                queue_recipe_book_seen_recipe_command(
                    counters,
                    net_commands,
                    bbb_protocol::packets::RecipeBookSeenRecipeCommand {
                        recipe: bbb_protocol::packets::RecipeDisplayId {
                            index: recipe_index,
                        },
                    },
                );
            }
            NetControlRequest::RenameItem { name } => {
                queue_rename_item_command(counters, net_commands, RenameItem { name });
            }
            NetControlRequest::SelectTrade { item } => {
                queue_select_trade_command(
                    counters,
                    net_commands,
                    bbb_protocol::packets::SelectTradeCommand { item },
                );
            }
            NetControlRequest::SignUpdate {
                x,
                y,
                z,
                is_front_text,
                lines,
            } => {
                queue_sign_update_command(
                    counters,
                    net_commands,
                    bbb_protocol::packets::SignUpdate {
                        pos: ProtocolBlockPos { x, y, z },
                        is_front_text,
                        lines,
                    },
                );
            }
            NetControlRequest::SelectBundleItem {
                slot_id,
                selected_item_index,
            } => {
                select_bundle_item(counters, world, net_commands, slot_id, selected_item_index);
            }
            NetControlRequest::ChatCommand { command } => {
                queue_chat_command(counters, net_commands, command);
            }
            NetControlRequest::CommandSuggestionRequest { id, command } => {
                queue_command_suggestion_request(counters, net_commands, id, command);
            }
            NetControlRequest::ContainerButtonClick {
                container_id,
                button_id,
            } => {
                queue_container_button_click_command(
                    counters,
                    net_commands,
                    container_id,
                    button_id,
                );
            }
            NetControlRequest::ContainerClick(click) => {
                queue_container_click_command(
                    counters,
                    net_commands,
                    protocol_container_click(click),
                );
            }
            NetControlRequest::ContainerClose { container_id } => {
                world.close_local_container(container_id);
                queue_container_close_request_command(counters, net_commands, container_id);
            }
            NetControlRequest::ContainerSlotStateChanged {
                slot_id,
                container_id,
                new_state,
            } => {
                queue_container_slot_state_changed_command(
                    counters,
                    net_commands,
                    slot_id,
                    container_id,
                    new_state,
                );
            }
        }
    }

    let mut code_of_conduct = code_of_conduct;
    for request in requests {
        match request {
            CodeOfConductControlRequest::Accept { remember } => {
                let Some(tx) = net_commands else {
                    continue;
                };
                if tx.try_send(NetCommand::AcceptCodeOfConduct).is_err() {
                    break;
                }
                if let Some(code_of_conduct) = code_of_conduct.as_deref_mut() {
                    let result = if remember {
                        code_of_conduct.persist_current_world_acceptance(world)
                    } else {
                        code_of_conduct.clear_connected_server_acceptance()
                    };
                    if let Err(err) = result {
                        tracing::warn!(
                            ?err,
                            remember,
                            "failed to update code-of-conduct acceptance store"
                        );
                    }
                }
            }
            CodeOfConductControlRequest::Decline => {
                if let Some(code_of_conduct) = code_of_conduct.as_deref_mut() {
                    if let Err(err) = code_of_conduct.clear_connected_server_acceptance() {
                        tracing::warn!(?err, "failed to clear code-of-conduct acceptance");
                    }
                }
                if let Some(tx) = net_commands {
                    if tx.try_send(NetCommand::Disconnect).is_err() {
                        break;
                    }
                }
            }
            CodeOfConductControlRequest::ClearAcceptance => {
                if let Some(code_of_conduct) = code_of_conduct.as_deref_mut() {
                    if let Err(err) = code_of_conduct.clear_connected_server_acceptance() {
                        tracing::warn!(?err, "failed to clear code-of-conduct acceptance");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn pump_control_net_requests_queues_chat_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::ChatCommand {
                command: "give @p minecraft:stone".to_string(),
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.chat_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(bbb_protocol::packets::ChatCommand {
                command: "give @p minecraft:stone".to_string()
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_command_suggestion_request() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.push(
            bbb_control::NetControlRequest::CommandSuggestionRequest {
                id: 18,
                command: "/give @p minecraft:stone".to_string(),
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.command_suggestion_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(bbb_protocol::packets::CommandSuggestionRequest {
                id: 18,
                command: "/give @p minecraft:stone".to_string(),
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_sets_held_slot_and_queues_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::SetHeldSlot { slot: 4 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(world.counters().held_slot_packets, 0);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(4));
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_sets_flying_when_allowed_and_queues_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::SetFlying { flying: true });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert!(world.local_player().abilities.unwrap().flying);
        assert_eq!(world.counters().player_abilities_packets, 1);
        assert_eq!(counters.player_abilities_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAbilities(bbb_protocol::packets::PlayerAbilitiesCommand {
                flying: true
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_does_not_set_flying_without_permission() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::SetFlying { flying: true });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert!(!world.local_player().abilities.unwrap().flying);
        assert_eq!(counters.player_abilities_commands_queued, 0);
        assert!(rx.try_recv().is_err());
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_place_recipe() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::PlaceRecipe {
                container_id: 7,
                recipe_index: 123,
                use_max_items: true,
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.place_recipe_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
                container_id: 7,
                recipe_index: 123,
                use_max_items: true,
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_recipe_book_commands() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.extend([
            bbb_control::NetControlRequest::ChangeRecipeBookSettings {
                book_type: bbb_control::RecipeBookTypeControl::BlastFurnace,
                open: true,
                filtering: false,
            },
            bbb_control::NetControlRequest::MarkRecipeSeen { recipe_index: 321 },
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
        assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RecipeBookChangeSettings(
                bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                    book_type: bbb_protocol::packets::RecipeBookType::BlastFurnace,
                    open: true,
                    filtering: false,
                }
            )
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RecipeBookSeenRecipe(bbb_protocol::packets::RecipeBookSeenRecipeCommand {
                recipe: bbb_protocol::packets::RecipeDisplayId { index: 321 },
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_sign_update() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::SignUpdate {
                x: -5,
                y: 70,
                z: 12,
                is_front_text: false,
                lines: [
                    "line 0".to_string(),
                    "line 1".to_string(),
                    "line 2".to_string(),
                    "line 3".to_string(),
                ],
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.sign_update_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SignUpdate(bbb_protocol::packets::SignUpdate {
                pos: bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                is_front_text: false,
                lines: [
                    "line 0".to_string(),
                    "line 1".to_string(),
                    "line 2".to_string(),
                    "line 3".to_string(),
                ],
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_rename_item() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::RenameItem {
                name: "Sharp Pick".to_string(),
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.rename_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: "Sharp Pick".to_string(),
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_select_trade() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::SelectTrade { item: 2 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.select_trade_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectTrade(bbb_protocol::packets::SelectTradeCommand { item: 2 })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_select_bundle_item() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.push(
            bbb_control::NetControlRequest::SelectBundleItem {
                slot_id: 12,
                selected_item_index: 3,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 12,
            item: bundle_item_stack(42, 1, 4),
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert_eq!(
            world.inventory().player_slots[0].local_selected_bundle_item_index,
            3
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(bbb_protocol::packets::SelectBundleItem {
                slot_id: 12,
                selected_item_index: 3,
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_rejects_invalid_select_bundle_item_index() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.push(
            bbb_control::NetControlRequest::SelectBundleItem {
                slot_id: 12,
                selected_item_index: -2,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 12,
            item: bundle_item_stack(42, 1, 4),
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.select_bundle_item_commands_queued, 0);
        assert_eq!(
            world.inventory().player_slots[0].local_selected_bundle_item_index,
            -1
        );
        assert!(rx.try_recv().is_err());
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_container_button_click() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.push(
            bbb_control::NetControlRequest::ContainerButtonClick {
                container_id: 7,
                button_id: 2,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(bbb_protocol::packets::ContainerButtonClick {
                container_id: 7,
                button_id: 2,
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_container_click() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.push(
            bbb_control::NetControlRequest::ContainerClick(
                bbb_control::ContainerClickControlRequest {
                    container_id: 7,
                    state_id: 33,
                    slot_num: 5,
                    button_num: 1,
                    input: bbb_control::ContainerInputControl::Pickup,
                    changed_slots: vec![bbb_control::ContainerChangedSlotControl {
                        slot: 5,
                        stack: bbb_control::HashedStackControl::Item {
                            item_id: 42,
                            count: 64,
                            components: bbb_control::HashedComponentPatchControl {
                                added_components: BTreeMap::from([(10, 0x0102_0304)]),
                                removed_components: BTreeSet::from([20]),
                            },
                        },
                    }],
                    carried_item: bbb_control::HashedStackControl::Empty,
                },
            ),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(bbb_protocol::packets::ContainerClick {
                container_id: 7,
                state_id: 33,
                slot_num: 5,
                button_num: 1,
                input: bbb_protocol::packets::ContainerInput::Pickup,
                changed_slots: BTreeMap::from([(
                    5,
                    bbb_protocol::packets::HashedStack::Item(
                        bbb_protocol::packets::HashedItemStack {
                            item_id: 42,
                            count: 64,
                            components: bbb_protocol::packets::HashedComponentPatch {
                                added_components: BTreeMap::from([(10, 0x0102_0304)]),
                                removed_components: BTreeSet::from([20]),
                            },
                        }
                    )
                )]),
                carried_item: bbb_protocol::packets::HashedStack::empty(),
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_container_close() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::ContainerClose { container_id: 7 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
                container_id: 7,
            })
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_closes_matching_world_container() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .net_requests
            .push(bbb_control::NetControlRequest::ContainerClose { container_id: 7 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 18,
            title: "Inventory".to_string(),
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert!(world.inventory().open_container.is_none());
        assert_eq!(world.counters().container_close_updates_received, 0);
        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
                container_id: 7,
            })
        );
    }

    #[test]
    fn pump_control_net_requests_queues_container_slot_state_changed() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot.write().unwrap().net_requests.push(
            bbb_control::NetControlRequest::ContainerSlotStateChanged {
                slot_id: 12,
                container_id: 7,
                new_state: true,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_slot_state_changed_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerSlotStateChanged(
                bbb_protocol::packets::ContainerSlotStateChanged {
                    slot_id: 12,
                    container_id: 7,
                    new_state: true,
                }
            )
        );
        assert!(snapshot.read().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_code_of_conduct_accept_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: false });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&snapshot, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        assert!(snapshot.read().unwrap().code_of_conduct_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_persists_current_code_of_conduct_hash() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: true });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let text = "Keep the server friendly.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());
        acceptance.set_connected_server(&options);
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &snapshot,
            &Some(tx.clone()),
            &mut counters,
            &mut world,
            Some(&mut acceptance),
        );

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(
            loaded.accepted_hash_for_options(&options),
            Some(bbb_world::code_of_conduct_text_hash(text))
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_non_persistent_accept_clears_existing_hash() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: false });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &snapshot,
            &Some(tx.clone()),
            &mut counters,
            &mut world,
            Some(&mut acceptance),
        );

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_decline_clears_hash_and_disconnects() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Decline);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &snapshot,
            &Some(tx.clone()),
            &mut counters,
            &mut world,
            Some(&mut acceptance),
        );

        assert_eq!(rx.try_recv().unwrap(), NetCommand::Disconnect);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_clear_acceptance_does_not_send_accept_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::ClearAcceptance);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &snapshot,
            &Some(tx.clone()),
            &mut counters,
            &mut world,
            Some(&mut acceptance),
        );

        assert!(matches!(
            rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = std::fs::remove_file(path);
    }

    fn bundle_item_stack(
        item_id: i32,
        count: i32,
        bundle_contents_item_count: usize,
    ) -> bbb_protocol::packets::ItemStackSummary {
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                bundle_contents_item_count: Some(bundle_contents_item_count),
                ..bbb_protocol::packets::DataComponentPatchSummary::default()
            },
        }
    }

    fn unique_code_of_conduct_store_path() -> std::path::PathBuf {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-runtime-code-of-conduct-{}-{id}-{nanos}.json",
            std::process::id()
        ))
    }
}
