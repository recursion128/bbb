use crate::control::{
    CodeOfConductControlRequest, ContainerClickControlRequest, ContainerInputControl,
    CreativeModeItemStackControl, CreativeModeSlotControlRequest, DifficultyControl,
    GameModeControl, HashedComponentPatchControl, HashedStackControl, InputKeyControl,
    InputMouseButtonControl, NetControlRequest, NetCounters, RecipeBookTypeControl,
};
use bbb_client_net::NetCommand;
#[cfg(test)]
use bbb_client_net::OutboundChatCommand;
use bbb_client_world::{ContainerClickSlotRequest, WorldStore};
use bbb_protocol::packets::{
    BlockEntityTagQuery, BlockPos as ProtocolBlockPos, ChangeDifficultyCommand,
    ChangeGameModeCommand, ContainerClick, ContainerInput, Difficulty, EditBook, EntityTagQuery,
    GameType, HashedComponentPatch, HashedItemStack, HashedStack, ItemStackSummary,
    LockDifficultyCommand, RecipeBookType, RenameItem, SeenAdvancements, SetBeacon,
    SetCreativeModeSlot, SpectateEntity, TeleportToEntity,
};
use tokio::sync::mpsc;

use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    code_of_conduct::CodeOfConductAcceptance,
    input::{
        queue_block_entity_tag_query_command, queue_change_difficulty_command,
        queue_change_game_mode_command, queue_chat_command, queue_chat_message_command,
        queue_command_suggestion_request, queue_container_button_click_command,
        queue_container_click_command, queue_container_close_request_command,
        queue_container_slot_state_changed_command, queue_edit_book_command,
        queue_entity_tag_query_command, queue_lock_difficulty_command,
        queue_perform_respawn_command, queue_place_recipe_command, queue_player_abilities_command,
        queue_recipe_book_change_settings_command, queue_recipe_book_seen_recipe_command,
        queue_rename_item_command, queue_request_stats_command, queue_seen_advancements_command,
        queue_select_trade_command, queue_set_beacon_command, queue_set_creative_mode_slot_command,
        queue_sign_update_command, queue_spectate_entity_command, queue_teleport_to_entity_command,
        select_bundle_item, select_hotbar_slot,
    },
};

pub(crate) fn input_mouse_button(button: InputMouseButtonControl) -> MouseButton {
    match button {
        InputMouseButtonControl::Left => MouseButton::Left,
        InputMouseButtonControl::Right => MouseButton::Right,
        InputMouseButtonControl::Middle => MouseButton::Middle,
    }
}

/// The default vanilla key binding for each semantic control-API key.
pub(crate) fn input_key_code(key: InputKeyControl) -> KeyCode {
    match key {
        InputKeyControl::Forward => KeyCode::KeyW,
        InputKeyControl::Backward => KeyCode::KeyS,
        InputKeyControl::Left => KeyCode::KeyA,
        InputKeyControl::Right => KeyCode::KeyD,
        InputKeyControl::Jump => KeyCode::Space,
        InputKeyControl::Sneak => KeyCode::ShiftLeft,
        InputKeyControl::Sprint => KeyCode::ControlLeft,
        InputKeyControl::Hotbar1 => KeyCode::Digit1,
        InputKeyControl::Hotbar2 => KeyCode::Digit2,
        InputKeyControl::Hotbar3 => KeyCode::Digit3,
        InputKeyControl::Hotbar4 => KeyCode::Digit4,
        InputKeyControl::Hotbar5 => KeyCode::Digit5,
        InputKeyControl::Hotbar6 => KeyCode::Digit6,
        InputKeyControl::Hotbar7 => KeyCode::Digit7,
        InputKeyControl::Hotbar8 => KeyCode::Digit8,
        InputKeyControl::Hotbar9 => KeyCode::Digit9,
        InputKeyControl::Inventory => KeyCode::KeyE,
        InputKeyControl::Chat => KeyCode::KeyT,
        InputKeyControl::Command => KeyCode::Slash,
        InputKeyControl::Enter => KeyCode::Enter,
        InputKeyControl::Escape => KeyCode::Escape,
        InputKeyControl::Up => KeyCode::ArrowUp,
        InputKeyControl::Down => KeyCode::ArrowDown,
        InputKeyControl::PageUp => KeyCode::PageUp,
        InputKeyControl::PageDown => KeyCode::PageDown,
        InputKeyControl::Backspace => KeyCode::Backspace,
        InputKeyControl::F1 => KeyCode::F1,
        InputKeyControl::F3 => KeyCode::F3,
        InputKeyControl::F4 => KeyCode::F4,
        InputKeyControl::F5 => KeyCode::F5,
        InputKeyControl::F6 => KeyCode::F6,
        InputKeyControl::F9 => KeyCode::F9,
        InputKeyControl::Tab => KeyCode::Tab,
        InputKeyControl::Drop => KeyCode::KeyQ,
        InputKeyControl::SwapOffhand => KeyCode::KeyF,
        InputKeyControl::Advancements => KeyCode::KeyL,
        InputKeyControl::Social => KeyCode::KeyP,
    }
}

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

fn world_container_click_slot(
    click: crate::control::ContainerClickSlotControlRequest,
) -> ContainerClickSlotRequest {
    ContainerClickSlotRequest {
        slot_num: click.slot_num,
        button_num: click.button_num,
        input: protocol_container_input(click.input),
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

fn protocol_creative_mode_slot(request: CreativeModeSlotControlRequest) -> SetCreativeModeSlot {
    let item = match request.item {
        CreativeModeItemStackControl::Empty => ItemStackSummary::empty(),
        CreativeModeItemStackControl::Item { item_id, count } => ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        },
    };
    SetCreativeModeSlot {
        slot_num: request.slot_num,
        item,
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

fn protocol_difficulty(difficulty: DifficultyControl) -> Difficulty {
    match difficulty {
        DifficultyControl::Peaceful => Difficulty::Peaceful,
        DifficultyControl::Easy => Difficulty::Easy,
        DifficultyControl::Normal => Difficulty::Normal,
        DifficultyControl::Hard => Difficulty::Hard,
    }
}

fn protocol_game_mode(game_mode: GameModeControl) -> GameType {
    match game_mode {
        GameModeControl::Survival => GameType::Survival,
        GameModeControl::Creative => GameType::Creative,
        GameModeControl::Adventure => GameType::Adventure,
        GameModeControl::Spectator => GameType::Spectator,
    }
}

pub(crate) fn apply_control_net_request(
    request: NetControlRequest,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    counters: &mut NetCounters,
    world: &mut WorldStore,
) {
    match request {
        NetControlRequest::ChangeDifficulty { difficulty } => {
            queue_change_difficulty_command(
                counters,
                net_commands,
                ChangeDifficultyCommand {
                    difficulty: protocol_difficulty(difficulty),
                },
            );
        }
        NetControlRequest::ChangeGameMode { game_mode } => {
            queue_change_game_mode_command(
                counters,
                net_commands,
                ChangeGameModeCommand {
                    game_mode: protocol_game_mode(game_mode),
                },
            );
        }
        NetControlRequest::LockDifficulty { locked } => {
            queue_lock_difficulty_command(counters, net_commands, LockDifficultyCommand { locked });
        }
        NetControlRequest::SetHeldSlot { slot } => {
            select_hotbar_slot(counters, world, net_commands, slot);
        }
        NetControlRequest::SetFlying { flying } => {
            queue_player_abilities_command(counters, world, net_commands, flying);
        }
        NetControlRequest::PerformRespawn => {
            queue_perform_respawn_command(counters, net_commands);
        }
        NetControlRequest::RequestStats => {
            queue_request_stats_command(counters, net_commands);
        }
        // Opens the game rules screen, which the input state owns.
        NetControlRequest::RequestGameRuleValues => {}
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
        NetControlRequest::EditBook { slot, pages, title } => {
            queue_edit_book_command(
                counters,
                net_commands,
                EditBook {
                    slot,
                    pages: pages
                        .into_iter()
                        .map(|text| text.encode_utf16().collect())
                        .collect(),
                    title: title.map(|text| text.encode_utf16().collect()),
                },
            );
        }
        NetControlRequest::RenameItem { name } => {
            queue_rename_item_command(counters, net_commands, RenameItem { name });
        }
        NetControlRequest::OpenAdvancementsTab { tab } => {
            queue_seen_advancements_command(
                counters,
                net_commands,
                SeenAdvancements::OpenedTab { tab },
            );
        }
        NetControlRequest::CloseAdvancementsScreen => {
            queue_seen_advancements_command(counters, net_commands, SeenAdvancements::ClosedScreen);
        }
        NetControlRequest::SelectTrade { item } => {
            queue_select_trade_command(
                counters,
                net_commands,
                bbb_protocol::packets::SelectTradeCommand { item },
            );
        }
        NetControlRequest::SetBeacon {
            primary_effect,
            secondary_effect,
        } => {
            queue_set_beacon_command(
                counters,
                net_commands,
                SetBeacon {
                    primary_effect,
                    secondary_effect,
                },
            );
        }
        NetControlRequest::SetCreativeModeSlot(request) => {
            queue_set_creative_mode_slot_command(
                counters,
                net_commands,
                protocol_creative_mode_slot(request),
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
            queue_chat_command(counters, world, net_commands, command);
        }
        NetControlRequest::ChatMessage { message } => {
            queue_chat_message_command(counters, world, net_commands, message);
        }
        NetControlRequest::CommandSuggestionRequest { id, command } => {
            queue_command_suggestion_request(counters, net_commands, id, command);
        }
        NetControlRequest::QueryBlockEntityTag {
            transaction_id,
            x,
            y,
            z,
        } => {
            queue_block_entity_tag_query_command(
                counters,
                net_commands,
                BlockEntityTagQuery {
                    transaction_id,
                    pos: ProtocolBlockPos { x, y, z },
                },
            );
        }
        NetControlRequest::QueryEntityTag {
            transaction_id,
            entity_id,
        } => {
            queue_entity_tag_query_command(
                counters,
                net_commands,
                EntityTagQuery {
                    transaction_id,
                    entity_id,
                },
            );
        }
        NetControlRequest::SpectateEntity { entity_id } => {
            queue_spectate_entity_command(counters, net_commands, SpectateEntity { entity_id });
        }
        NetControlRequest::TeleportToEntity { uuid } => {
            queue_teleport_to_entity_command(counters, net_commands, TeleportToEntity { uuid });
        }
        NetControlRequest::ContainerButtonClick {
            container_id,
            button_id,
        } => {
            queue_container_button_click_command(counters, net_commands, container_id, button_id);
        }
        NetControlRequest::ContainerClick(click) => {
            queue_container_click_command(counters, net_commands, protocol_container_click(click));
        }
        NetControlRequest::ContainerClickSlot(click) => {
            if let Ok(packet) = world.build_container_click_slot(world_container_click_slot(click))
            {
                queue_container_click_command(counters, net_commands, packet);
            }
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

pub(crate) fn apply_code_of_conduct_request(
    request: CodeOfConductControlRequest,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    world: &WorldStore,
    code_of_conduct: Option<&mut CodeOfConductAcceptance>,
) {
    match request {
        CodeOfConductControlRequest::Accept { remember } => {
            let Some(tx) = net_commands else {
                return;
            };
            if tx.try_send(NetCommand::AcceptCodeOfConduct).is_err() {
                return;
            }
            if let Some(code_of_conduct) = code_of_conduct {
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
            if let Some(code_of_conduct) = code_of_conduct {
                if let Err(err) = code_of_conduct.clear_connected_server_acceptance() {
                    tracing::warn!(?err, "failed to clear code-of-conduct acceptance");
                }
            }
            if let Some(tx) = net_commands {
                if tx.try_send(NetCommand::Disconnect).is_err() {}
            }
        }
        CodeOfConductControlRequest::ClearAcceptance => {
            if let Some(code_of_conduct) = code_of_conduct {
                if let Err(err) = code_of_conduct.clear_connected_server_acceptance() {
                    tracing::warn!(?err, "failed to clear code-of-conduct acceptance");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Mutex, MutexGuard};

    use super::*;
    use bbb_protocol::menu_types::{VANILLA_MENU_TYPE_GENERIC_9X3_ID, VANILLA_MENU_TYPE_LOOM_ID};
    use bbb_protocol::packets::{
        CommandArgumentParser, CommandNode, CommandNodeType, Commands,
        VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
    };
    use uuid::Uuid;

    #[derive(Default)]
    struct TestPendingControlRequests {
        code_of_conduct_requests: Vec<CodeOfConductControlRequest>,
        net_requests: Vec<NetControlRequest>,
    }

    #[derive(Default)]
    struct TestControlRequests {
        requests: Mutex<TestPendingControlRequests>,
    }

    impl TestControlRequests {
        fn lock(&self) -> std::sync::LockResult<MutexGuard<'_, TestPendingControlRequests>> {
            self.requests.lock()
        }
    }

    fn pump_control_net_requests(
        control_requests: &TestControlRequests,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        code_of_conduct: Option<&mut CodeOfConductAcceptance>,
    ) {
        let (code_of_conduct_requests, net_requests) = control_requests
            .lock()
            .map(|mut guard| {
                (
                    std::mem::take(&mut guard.code_of_conduct_requests),
                    std::mem::take(&mut guard.net_requests),
                )
            })
            .unwrap_or_default();
        for request in net_requests {
            apply_control_net_request(request, net_commands, counters, world);
        }
        let mut code_of_conduct = code_of_conduct;
        for request in code_of_conduct_requests {
            apply_code_of_conduct_request(
                request,
                net_commands,
                world,
                code_of_conduct.as_deref_mut(),
            );
        }
    }

    #[test]
    fn pump_control_net_requests_queues_chat_command() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ChatCommand {
                command: "give @p minecraft:stone".to_string(),
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.chat_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(OutboundChatCommand::unsigned("give @p minecraft:stone"))
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_chat_message() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ChatMessage {
                message: "hello from the gate".to_string(),
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.chat_message_commands_queued, 1);
        let Ok(NetCommand::ChatMessage(message)) = rx.try_recv() else {
            panic!("expected a queued outbound chat message");
        };
        assert_eq!(message.message, "hello from the gate");
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_signed_chat_command_for_signable_argument() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ChatCommand {
                command: "say hello".to_string(),
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_commands(signable_message_command_tree());
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.chat_command_commands_queued, 1);
        match rx.try_recv().unwrap() {
            NetCommand::ChatCommand(packet) => {
                assert_eq!(packet.command, "say hello");
                let signing = packet.signing.expect("signable command signing context");
                assert!(signing.timestamp_millis > 0);
                assert_ne!(signing.salt, 0);
                assert_eq!(signing.arguments[0].name, "message");
                assert_eq!(signing.arguments[0].value, "hello");
            }
            command => panic!("expected chat command request, got {command:?}"),
        }
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_command_suggestion_request() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::CommandSuggestionRequest {
                id: 18,
                command: "/give @p minecraft:stone".to_string(),
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.command_suggestion_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(bbb_protocol::packets::CommandSuggestionRequest {
                id: 18,
                command: "/give @p minecraft:stone".to_string().into(),
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_tag_query_commands() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.extend([
            crate::control::NetControlRequest::QueryBlockEntityTag {
                transaction_id: 11,
                x: -5,
                y: 70,
                z: 12,
            },
            crate::control::NetControlRequest::QueryEntityTag {
                transaction_id: 12,
                entity_id: 123,
            },
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.block_entity_tag_query_commands_queued, 1);
        assert_eq!(counters.entity_tag_query_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::BlockEntityTagQuery(BlockEntityTagQuery {
                transaction_id: 11,
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::EntityTagQuery(EntityTagQuery {
                transaction_id: 12,
                entity_id: 123,
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_spectator_entity_commands() {
        let requests = TestControlRequests::default();
        let uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
        requests.lock().unwrap().net_requests.extend([
            crate::control::NetControlRequest::SpectateEntity { entity_id: 1234 },
            crate::control::NetControlRequest::TeleportToEntity { uuid },
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.spectate_entity_commands_queued, 1);
        assert_eq!(counters.teleport_to_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SpectateEntity(SpectateEntity { entity_id: 1234 })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::TeleportToEntity(TeleportToEntity { uuid })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_sets_held_slot_and_queues_command() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::SetHeldSlot { slot: 4 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(world.counters().held_slot_packets, 0);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(4));
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_sets_flying_when_allowed_and_queues_command() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::SetFlying { flying: true });
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

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert!(world.local_player().abilities.unwrap().flying);
        assert_eq!(world.counters().player_abilities_packets, 1);
        assert_eq!(counters.player_abilities_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAbilities(bbb_protocol::packets::PlayerAbilitiesCommand {
                flying: true
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_does_not_set_flying_without_permission() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::SetFlying { flying: true });
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

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert!(!world.local_player().abilities.unwrap().flying);
        assert_eq!(counters.player_abilities_commands_queued, 0);
        assert!(rx.try_recv().is_err());
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_perform_respawn() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::PerformRespawn);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.perform_respawn_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::PerformRespawn);
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_place_recipe() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::PlaceRecipe {
                container_id: 7,
                recipe_index: 123,
                use_max_items: true,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.place_recipe_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
                container_id: 7,
                recipe_index: 123,
                use_max_items: true,
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_difficulty_commands() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.extend([
            crate::control::NetControlRequest::ChangeDifficulty {
                difficulty: crate::control::DifficultyControl::Hard,
            },
            crate::control::NetControlRequest::ChangeGameMode {
                game_mode: crate::control::GameModeControl::Adventure,
            },
            crate::control::NetControlRequest::LockDifficulty { locked: true },
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(3);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(world.difficulty().difficulty, "normal");
        assert_eq!(world.gameplay().game_type, GameType::Survival.id());
        assert_eq!(counters.change_difficulty_commands_queued, 1);
        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(counters.lock_difficulty_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeDifficulty(ChangeDifficultyCommand {
                difficulty: Difficulty::Hard,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(ChangeGameModeCommand {
                game_mode: GameType::Adventure,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::LockDifficulty(LockDifficultyCommand { locked: true })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_recipe_book_commands() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.extend([
            crate::control::NetControlRequest::ChangeRecipeBookSettings {
                book_type: crate::control::RecipeBookTypeControl::BlastFurnace,
                open: true,
                filtering: false,
            },
            crate::control::NetControlRequest::MarkRecipeSeen { recipe_index: 321 },
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

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
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_edit_book() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::EditBook {
                slot: 5,
                pages: vec!["first page".to_string()],
                title: Some("Field Notes".to_string()),
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.edit_book_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::EditBook(EditBook {
                slot: 5,
                pages: vec!["first page".encode_utf16().collect()],
                title: Some("Field Notes".encode_utf16().collect()),
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_sign_update() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::SignUpdate {
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

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

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
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_rename_item() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::RenameItem {
                name: "Sharp Pick".to_string(),
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.rename_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: "Sharp Pick".to_string(),
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_seen_advancements() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::OpenAdvancementsTab {
                tab: "minecraft:story/root".to_string(),
            },
        );
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::CloseAdvancementsScreen);
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.advancements_seen_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
                tab: "minecraft:story/root".to_string(),
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_select_trade() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::SelectTrade { item: 2 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.select_trade_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectTrade(bbb_protocol::packets::SelectTradeCommand { item: 2 })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_select_bundle_item() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::SelectBundleItem {
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

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

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
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_rejects_invalid_select_bundle_item_index() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::SelectBundleItem {
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

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.select_bundle_item_commands_queued, 0);
        assert_eq!(
            world.inventory().player_slots[0].local_selected_bundle_item_index,
            -1
        );
        assert!(rx.try_recv().is_err());
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_set_beacon() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::SetBeacon {
                primary_effect: Some(1),
                secondary_effect: None,
            });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.set_beacon_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SetBeacon(SetBeacon {
                primary_effect: Some(1),
                secondary_effect: None,
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_set_creative_mode_slot() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::SetCreativeModeSlot(
                crate::control::CreativeModeSlotControlRequest {
                    slot_num: 36,
                    item: crate::control::CreativeModeItemStackControl::Item {
                        item_id: 42,
                        count: 64,
                    },
                },
            ),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.set_creative_mode_slot_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 64,
                    component_patch: Default::default(),
                },
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_container_button_click() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ContainerButtonClick {
                container_id: 7,
                button_id: 2,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(bbb_protocol::packets::ContainerButtonClick {
                container_id: 7,
                button_id: 2,
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_container_click() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ContainerClick(
                crate::control::ContainerClickControlRequest {
                    container_id: 7,
                    state_id: 33,
                    slot_num: 5,
                    button_num: 1,
                    input: crate::control::ContainerInputControl::Pickup,
                    changed_slots: vec![crate::control::types::ContainerChangedSlotControl {
                        slot: 5,
                        stack: crate::control::HashedStackControl::Item {
                            item_id: 42,
                            count: 64,
                            components: crate::control::HashedComponentPatchControl {
                                added_components: BTreeMap::from([(10, 0x0102_0304)]),
                                removed_components: BTreeSet::from([20]),
                            },
                        },
                    }],
                    carried_item: crate::control::HashedStackControl::Empty,
                },
            ),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

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
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_builds_container_click_from_world_inventory() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ContainerClickSlot(
                crate::control::ContainerClickSlotControlRequest {
                    slot_num: 1,
                    button_num: 0,
                    input: crate::control::ContainerInputControl::Pickup,
                },
            ),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
            title: bbb_protocol::RichTextComponent::plain("Chest"),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 13,
            items: vec![
                bbb_protocol::packets::ItemStackSummary::empty(),
                bbb_protocol::packets::ItemStackSummary {
                    item_id: Some(42),
                    count: 3,
                    component_patch: Default::default(),
                },
            ],
            carried_item: bbb_protocol::packets::ItemStackSummary {
                item_id: Some(99),
                count: 1,
                component_patch: Default::default(),
            },
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(bbb_protocol::packets::ContainerClick {
                container_id: 7,
                state_id: 13,
                slot_num: 1,
                button_num: 0,
                input: bbb_protocol::packets::ContainerInput::Pickup,
                changed_slots: BTreeMap::new(),
                carried_item: bbb_protocol::packets::HashedStack::Item(
                    bbb_protocol::packets::HashedItemStack {
                        item_id: 99,
                        count: 1,
                        components: bbb_protocol::packets::HashedComponentPatch::default(),
                    }
                ),
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_builds_container_zero_click_when_local_inventory_is_open() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ContainerClickSlot(
                crate::control::ContainerClickSlotControlRequest {
                    slot_num: 36,
                    button_num: 0,
                    input: crate::control::ContainerInputControl::Pickup,
                },
            ),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: bbb_protocol::packets::ItemStackSummary {
                item_id: Some(42),
                count: 3,
                component_patch: Default::default(),
            },
        });
        world.apply_set_cursor_item(bbb_protocol::packets::SetCursorItem {
            item: bbb_protocol::packets::ItemStackSummary {
                item_id: Some(99),
                count: 1,
                component_patch: Default::default(),
            },
        });
        assert!(world.open_local_inventory());
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(bbb_protocol::packets::ContainerClick {
                container_id: 0,
                state_id: 0,
                slot_num: 36,
                button_num: 0,
                input: bbb_protocol::packets::ContainerInput::Pickup,
                changed_slots: BTreeMap::new(),
                carried_item: bbb_protocol::packets::HashedStack::Item(
                    bbb_protocol::packets::HashedItemStack {
                        item_id: 99,
                        count: 1,
                        components: bbb_protocol::packets::HashedComponentPatch::default(),
                    }
                ),
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_skips_container_click_slot_without_open_container() {
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ContainerClickSlot(
                crate::control::ContainerClickSlotControlRequest {
                    slot_num: 1,
                    button_num: 0,
                    input: crate::control::ContainerInputControl::Pickup,
                },
            ),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_click_commands_queued, 0);
        assert!(rx.try_recv().is_err());
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_container_close() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::ContainerClose { container_id: 7 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
                container_id: 7,
            })
        );
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_closes_matching_world_container() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .net_requests
            .push(crate::control::NetControlRequest::ContainerClose { container_id: 7 });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
            title: bbb_protocol::RichTextComponent::plain("Inventory"),
        });
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

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
        let requests = TestControlRequests::default();
        requests.lock().unwrap().net_requests.push(
            crate::control::NetControlRequest::ContainerSlotStateChanged {
                slot_id: 12,
                container_id: 7,
                new_state: true,
            },
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

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
        assert!(requests.lock().unwrap().net_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_queues_code_of_conduct_accept_command() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: false });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        pump_control_net_requests(&requests, &Some(tx), &mut counters, &mut world, None);

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        assert!(requests.lock().unwrap().code_of_conduct_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_persists_current_code_of_conduct_hash() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: true });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options =
            bbb_client_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let text = "Keep the server friendly.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());
        acceptance.set_connected_server(&options);
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &requests,
            &Some(tx.clone()),
            &mut counters,
            &mut world,
            Some(&mut acceptance),
        );

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(
            loaded.accepted_hash_for_options(&options),
            Some(bbb_client_world::code_of_conduct_text_hash(text))
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_non_persistent_accept_clears_existing_hash() {
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: false });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options =
            bbb_client_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &requests,
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
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Decline);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options =
            bbb_client_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &requests,
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
        let requests = TestControlRequests::default();
        requests
            .lock()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::ClearAcceptance);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options =
            bbb_client_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();
        let mut counters = NetCounters::default();

        pump_control_net_requests(
            &requests,
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
        contents_count: usize,
    ) -> bbb_protocol::packets::ItemStackSummary {
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
                ],
                bundle_contents_items: (0..contents_count)
                    .map(|_| bbb_protocol::packets::ItemStackTemplateSummary {
                        item_id: bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
                        count: 1,
                        component_patch: Default::default(),
                    })
                    .collect(),
                ..bbb_protocol::packets::DataComponentPatchSummary::default()
            }.into(),
        }
    }

    fn signable_message_command_tree() -> Commands {
        Commands {
            root_index: 0,
            nodes: vec![
                CommandNode {
                    node_type: CommandNodeType::Root,
                    flags: 0,
                    children: vec![1],
                    redirect: None,
                    name: None,
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![2],
                    redirect: None,
                    name: Some("say".to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 6,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("message".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                        name: "minecraft:message".to_string(),
                        properties: Vec::new(),
                    }),
                    suggestions: None,
                    executable: true,
                    restricted: false,
                },
            ],
        }
    }

    fn unique_code_of_conduct_store_path() -> std::path::PathBuf {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = bbb_client_clock::SystemTime::now()
            .duration_since(bbb_client_clock::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-runtime-code-of-conduct-{}-{id}-{nanos}.json",
            std::process::id()
        ))
    }
}
