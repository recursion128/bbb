use bbb_control::NetCounters;
use bbb_net::{NetCommand, VehicleMoveCommand};
use bbb_protocol::packets::{
    AttackEntity, ChangeDifficultyCommand, ChatCommand, CommandSuggestionRequest,
    ContainerButtonClick, ContainerClick, ContainerCloseRequest, ContainerSlotStateChanged,
    Direction as ProtocolDirection, EditBook, InteractEntity, InteractionHand,
    LockDifficultyCommand, PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand,
    PlayerAbilitiesCommand, PlayerAction, PlayerActionKind, PlayerCommand, PlayerCommandAction,
    PlayerInput, RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RenameItem,
    SeenAdvancements, SelectBundleItem, SelectTradeCommand, SetBeacon, SignUpdate, UseItem,
    UseItemOn, Vec3d as ProtocolVec3d,
};
use bbb_world::{BlockPos, WorldStore};
use tokio::sync::mpsc;
use winit::keyboard::KeyCode;

use crate::crosshair::{
    protocol_block_hit_result_from_crosshair_hit, protocol_block_pos_from_world, CrosshairBlockHit,
};
pub(super) fn queue_player_input_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    input: PlayerInput,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::PlayerInput(input)).is_ok() {
            counters.player_input_commands_queued += 1;
        }
    }
}

pub(super) fn queue_sprint_command(
    counters: &mut NetCounters,
    world: &WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    sprinting: bool,
) {
    let action = if sprinting {
        PlayerCommandAction::StartSprinting
    } else {
        PlayerCommandAction::StopSprinting
    };
    queue_player_command_action(counters, world, net_commands, action, 0);
}

pub(crate) fn queue_player_abilities_command(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    flying: bool,
) -> bool {
    if !world.set_local_flying(flying) {
        return false;
    }
    if let Some(tx) = net_commands {
        if tx
            .try_send(NetCommand::PlayerAbilities(PlayerAbilitiesCommand {
                flying,
            }))
            .is_ok()
        {
            counters.player_abilities_commands_queued += 1;
        }
    }
    true
}

pub(crate) fn queue_place_recipe_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: PlaceRecipeCommand,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::PlaceRecipe(command)).is_ok() {
            counters.place_recipe_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_recipe_book_change_settings_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: RecipeBookChangeSettingsCommand,
) {
    if let Some(tx) = net_commands {
        if tx
            .try_send(NetCommand::RecipeBookChangeSettings(command))
            .is_ok()
        {
            counters.recipe_book_change_settings_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_recipe_book_seen_recipe_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: RecipeBookSeenRecipeCommand,
) {
    if let Some(tx) = net_commands {
        if tx
            .try_send(NetCommand::RecipeBookSeenRecipe(command))
            .is_ok()
        {
            counters.recipe_book_seen_recipe_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_edit_book_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: EditBook,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::EditBook(command)).is_ok() {
            counters.edit_book_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_change_difficulty_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: ChangeDifficultyCommand,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::ChangeDifficulty(command)).is_ok() {
            counters.change_difficulty_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_lock_difficulty_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: LockDifficultyCommand,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::LockDifficulty(command)).is_ok() {
            counters.lock_difficulty_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_rename_item_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: RenameItem,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::RenameItem(command)).is_ok() {
            counters.rename_item_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_seen_advancements_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: SeenAdvancements,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::SeenAdvancements(command)).is_ok() {
            counters.advancements_seen_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_select_trade_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: SelectTradeCommand,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::SelectTrade(command)).is_ok() {
            counters.select_trade_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_set_beacon_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: SetBeacon,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::SetBeacon(command)).is_ok() {
            counters.set_beacon_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_sign_update_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: SignUpdate,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::SignUpdate(command)).is_ok() {
            counters.sign_update_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_select_bundle_item_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
    selected_item_index: i32,
) -> bool {
    if selected_item_index < -1 {
        return false;
    }

    if let Some(tx) = net_commands {
        let packet = SelectBundleItem {
            slot_id,
            selected_item_index,
        };
        if tx.try_send(NetCommand::SelectBundleItem(packet)).is_ok() {
            counters.select_bundle_item_commands_queued += 1;
            return true;
        }
    }

    false
}

pub(super) fn queue_player_command_action(
    counters: &mut NetCounters,
    world: &WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    action: PlayerCommandAction,
    data: i32,
) {
    let (Some(tx), Some(entity_id)) = (net_commands, world.local_player_id()) else {
        return;
    };
    let command = PlayerCommand {
        entity_id,
        action,
        data,
    };
    if tx.try_send(NetCommand::PlayerCommand(command)).is_ok() {
        counters.player_command_commands_queued += 1;
    }
}

pub(super) fn hotbar_slot_for_key(code: KeyCode) -> Option<u8> {
    match code {
        KeyCode::Digit1 => Some(0),
        KeyCode::Digit2 => Some(1),
        KeyCode::Digit3 => Some(2),
        KeyCode::Digit4 => Some(3),
        KeyCode::Digit5 => Some(4),
        KeyCode::Digit6 => Some(5),
        KeyCode::Digit7 => Some(6),
        KeyCode::Digit8 => Some(7),
        KeyCode::Digit9 => Some(8),
        _ => None,
    }
}

pub(crate) fn select_hotbar_slot(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot: u8,
) {
    let slot = slot.min(8);
    if !world.set_local_selected_hotbar_slot(slot) {
        return;
    }
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::SetHeldSlot(slot)).is_ok() {
            counters.held_slot_commands_queued += 1;
        }
    }
}

pub(super) fn hotbar_slot_for_scroll(wheel: i32, current_slot: u8) -> Option<u8> {
    let step = wheel.signum();
    if step == 0 {
        return None;
    }

    let limit = 9;
    let mut slot = i32::from(current_slot.min(8)) - step;
    slot = slot.max(-1);
    while slot < 0 {
        slot += limit;
    }
    while slot >= limit {
        slot -= limit;
    }
    Some(slot as u8)
}

pub(super) fn queue_player_action_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    action_kind: PlayerActionKind,
    pos: BlockPos,
    direction: ProtocolDirection,
    sequence: i32,
) {
    let Some(tx) = net_commands else {
        return;
    };
    let action = PlayerAction {
        action: action_kind,
        pos: protocol_block_pos_from_world(pos),
        direction,
        sequence,
    };
    if tx.try_send(NetCommand::PlayerAction(action)).is_ok() {
        counters.player_action_commands_queued += 1;
    }
}

pub(crate) fn queue_chat_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    command: impl Into<String>,
) {
    if let Some(tx) = net_commands {
        let packet = ChatCommand {
            command: command.into(),
        };
        if tx.try_send(NetCommand::ChatCommand(packet)).is_ok() {
            counters.chat_command_commands_queued += 1;
        }
    }
}

pub(super) fn queue_container_close_command(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> bool {
    let Some(container_id) = world
        .inventory()
        .open_container
        .as_ref()
        .map(|container| container.container_id)
    else {
        return false;
    };
    if !world.close_local_container(container_id) {
        return false;
    }

    queue_container_close_request_command(counters, net_commands, container_id);
    true
}

pub(crate) fn queue_container_close_request_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    container_id: i32,
) {
    if let Some(tx) = net_commands {
        let packet = ContainerCloseRequest { container_id };
        if tx.try_send(NetCommand::ContainerClose(packet)).is_ok() {
            counters.container_close_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_container_button_click_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    container_id: i32,
    button_id: i32,
) {
    if let Some(tx) = net_commands {
        let packet = ContainerButtonClick {
            container_id,
            button_id,
        };
        if tx
            .try_send(NetCommand::ContainerButtonClick(packet))
            .is_ok()
        {
            counters.container_button_click_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_container_click_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    packet: ContainerClick,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::ContainerClick(packet)).is_ok() {
            counters.container_click_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_container_slot_state_changed_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    slot_id: i32,
    container_id: i32,
    new_state: bool,
) {
    if let Some(tx) = net_commands {
        let packet = ContainerSlotStateChanged {
            slot_id,
            container_id,
            new_state,
        };
        if tx
            .try_send(NetCommand::ContainerSlotStateChanged(packet))
            .is_ok()
        {
            counters.container_slot_state_changed_commands_queued += 1;
        }
    }
}

pub(super) fn queue_attack_entity_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_id: i32,
) {
    if let Some(tx) = net_commands {
        let packet = AttackEntity { entity_id };
        if tx.try_send(NetCommand::AttackEntity(packet)).is_ok() {
            counters.attack_entity_commands_queued += 1;
        }
    }
}

pub(super) fn queue_interact_entity_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_id: i32,
    hand: InteractionHand,
    location: ProtocolVec3d,
    using_secondary_action: bool,
) {
    if let Some(tx) = net_commands {
        let packet = InteractEntity {
            entity_id,
            hand,
            location,
            using_secondary_action,
        };
        if tx.try_send(NetCommand::InteractEntity(packet)).is_ok() {
            counters.interact_entity_commands_queued += 1;
        }
    }
}

pub(super) fn queue_zero_pos_player_action_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    action_kind: PlayerActionKind,
) {
    queue_player_action_command(
        counters,
        net_commands,
        action_kind,
        BlockPos { x: 0, y: 0, z: 0 },
        ProtocolDirection::Down,
        0,
    );
}

pub(super) fn queue_swing_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hand: InteractionHand,
) {
    if let Some(tx) = net_commands {
        if tx.try_send(NetCommand::Swing(hand)).is_ok() {
            counters.swing_commands_queued += 1;
        }
    }
}

pub(super) fn queue_use_item_on_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
    sequence: i32,
) {
    if let Some(tx) = net_commands {
        let packet = UseItemOn {
            hand: InteractionHand::MainHand,
            hit: protocol_block_hit_result_from_crosshair_hit(hit),
            sequence,
        };
        if tx.try_send(NetCommand::UseItemOn(packet)).is_ok() {
            counters.use_item_on_commands_queued += 1;
        }
    }
}

pub(super) fn queue_use_item_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hand: InteractionHand,
    y_rot: f32,
    x_rot: f32,
    sequence: i32,
) -> bool {
    if let Some(tx) = net_commands {
        let packet = UseItem {
            hand,
            sequence,
            y_rot,
            x_rot,
        };
        if tx.try_send(NetCommand::UseItem(packet)).is_ok() {
            counters.use_item_commands_queued += 1;
            return true;
        }
    }
    false
}

pub(super) fn queue_pick_item_from_block_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    pos: BlockPos,
    include_data: bool,
) {
    if let Some(tx) = net_commands {
        let packet = PickItemFromBlock {
            pos: protocol_block_pos_from_world(pos),
            include_data,
        };
        if tx.try_send(NetCommand::PickItemFromBlock(packet)).is_ok() {
            counters.pick_item_from_block_commands_queued += 1;
        }
    }
}

pub(super) fn queue_pick_item_from_entity_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_id: i32,
    include_data: bool,
) {
    if let Some(tx) = net_commands {
        let packet = PickItemFromEntity {
            entity_id,
            include_data,
        };
        if tx.try_send(NetCommand::PickItemFromEntity(packet)).is_ok() {
            counters.pick_item_from_entity_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_command_suggestion_request(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    id: i32,
    command: impl Into<String>,
) {
    if let Some(tx) = net_commands {
        let request = CommandSuggestionRequest {
            id,
            command: command.into(),
        };
        if tx
            .try_send(NetCommand::CommandSuggestionRequest(request))
            .is_ok()
        {
            counters.command_suggestion_commands_queued += 1;
        }
    }
}

pub(crate) fn queue_vehicle_move_command(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    report: bbb_world::VehicleMoveReport,
) {
    if let Some(tx) = net_commands {
        let command = VehicleMoveCommand {
            position: ProtocolVec3d {
                x: report.position.x,
                y: report.position.y,
                z: report.position.z,
            },
            y_rot: report.y_rot,
            x_rot: report.x_rot,
            on_ground: report.on_ground,
        };
        if tx.try_send(NetCommand::MoveVehicle(command)).is_ok() {
            counters.move_vehicle_commands_queued += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use bbb_protocol::packets::{
        AttackEntity, BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
        ChatCommand, CommandSuggestionRequest, ContainerButtonClick, ContainerSlotStateChanged,
        Direction as ProtocolDirection, InteractEntity, InteractionHand, PickItemFromBlock,
        PickItemFromEntity, PlayerAction, PlayerActionKind, RecipeBookType, RecipeDisplayId,
        UseItemOn,
    };
    use bbb_protocol::packets::{
        ContainerInput, HashedComponentPatch, HashedItemStack, HashedStack,
    };
    use bbb_world::BlockPos;

    use crate::crosshair::CrosshairBlockHit;

    #[test]
    fn hotbar_slot_for_scroll_matches_vanilla_direction_and_wrap() {
        assert_eq!(hotbar_slot_for_scroll(1, 0), Some(8));
        assert_eq!(hotbar_slot_for_scroll(-1, 8), Some(0));
        assert_eq!(hotbar_slot_for_scroll(2, 4), Some(3));
        assert_eq!(hotbar_slot_for_scroll(-2, 4), Some(5));
        assert_eq!(hotbar_slot_for_scroll(0, 4), None);
    }

    #[test]
    fn queues_command_suggestion_request() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_command_suggestion_request(&mut counters, &commands, 18, "/give @p minecraft:stone");

        assert_eq!(counters.command_suggestion_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id: 18,
                command: "/give @p minecraft:stone".to_string(),
            })
        );
    }

    #[test]
    fn queues_chat_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_chat_command(&mut counters, &commands, "give @p minecraft:stone");

        assert_eq!(counters.chat_command_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(ChatCommand {
                command: "give @p minecraft:stone".to_string(),
            })
        );
    }

    #[test]
    fn queues_recipe_book_commands() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_recipe_book_change_settings_command(
            &mut counters,
            &commands,
            RecipeBookChangeSettingsCommand {
                book_type: RecipeBookType::Furnace,
                open: true,
                filtering: false,
            },
        );
        queue_recipe_book_seen_recipe_command(
            &mut counters,
            &commands,
            RecipeBookSeenRecipeCommand {
                recipe: RecipeDisplayId { index: 321 },
            },
        );

        assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
        assert_eq!(counters.recipe_book_seen_recipe_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
                book_type: RecipeBookType::Furnace,
                open: true,
                filtering: false,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand {
                recipe: RecipeDisplayId { index: 321 },
            })
        );
    }

    #[test]
    fn queues_edit_book_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let command = EditBook {
            slot: 5,
            pages: vec!["first page".to_string()],
            title: Some("Field Notes".to_string()),
        };

        queue_edit_book_command(&mut counters, &commands, command.clone());

        assert_eq!(counters.edit_book_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::EditBook(command));
    }

    #[test]
    fn queues_difficulty_commands() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_change_difficulty_command(
            &mut counters,
            &commands,
            ChangeDifficultyCommand {
                difficulty: bbb_protocol::packets::Difficulty::Hard,
            },
        );
        queue_lock_difficulty_command(
            &mut counters,
            &commands,
            LockDifficultyCommand { locked: true },
        );

        assert_eq!(counters.change_difficulty_commands_queued, 1);
        assert_eq!(counters.lock_difficulty_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeDifficulty(ChangeDifficultyCommand {
                difficulty: bbb_protocol::packets::Difficulty::Hard,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::LockDifficulty(LockDifficultyCommand { locked: true })
        );
    }

    #[test]
    fn queues_rename_item_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let command = RenameItem {
            name: "Sharp Pick".to_string(),
        };

        queue_rename_item_command(&mut counters, &commands, command.clone());

        assert_eq!(counters.rename_item_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::RenameItem(command));
    }

    #[test]
    fn queues_seen_advancements_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let command = SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string(),
        };

        queue_seen_advancements_command(&mut counters, &commands, command.clone());

        assert_eq!(counters.advancements_seen_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SeenAdvancements(command)
        );
    }

    #[test]
    fn queues_set_beacon_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let command = SetBeacon {
            primary_effect: Some(1),
            secondary_effect: None,
        };

        queue_set_beacon_command(&mut counters, &commands, command);

        assert_eq!(counters.set_beacon_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetBeacon(command));
    }

    #[test]
    fn queues_sign_update_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let command = SignUpdate {
            pos: ProtocolBlockPos {
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
        };

        queue_sign_update_command(&mut counters, &commands, command.clone());

        assert_eq!(counters.sign_update_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SignUpdate(command));
    }

    #[test]
    fn queues_container_button_click_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_container_button_click_command(&mut counters, &commands, 7, 2);

        assert_eq!(counters.container_button_click_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerButtonClick(ContainerButtonClick {
                container_id: 7,
                button_id: 2,
            })
        );
    }

    #[test]
    fn queues_container_close_request_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_container_close_request_command(&mut counters, &commands, 7);

        assert_eq!(counters.container_close_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
        );
    }

    #[test]
    fn queues_container_click_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let packet = ContainerClick {
            container_id: 7,
            state_id: 33,
            slot_num: 5,
            button_num: 1,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::from([(
                5,
                HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 64,
                    components: HashedComponentPatch {
                        added_components: BTreeMap::from([(10, 0x0102_0304)]),
                        removed_components: BTreeSet::from([20]),
                    },
                }),
            )]),
            carried_item: HashedStack::empty(),
        };

        queue_container_click_command(&mut counters, &commands, packet.clone());

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::ContainerClick(packet));
    }

    #[test]
    fn queues_container_slot_state_changed_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_container_slot_state_changed_command(&mut counters, &commands, 12, 7, true);

        assert_eq!(counters.container_slot_state_changed_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerSlotStateChanged(ContainerSlotStateChanged {
                slot_id: 12,
                container_id: 7,
                new_state: true,
            })
        );
    }

    #[test]
    fn queues_select_bundle_item_command_and_rejects_invalid_index() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        assert!(queue_select_bundle_item_command(
            &mut counters,
            &commands,
            12,
            3
        ));
        assert!(queue_select_bundle_item_command(
            &mut counters,
            &commands,
            12,
            -1
        ));
        assert!(!queue_select_bundle_item_command(
            &mut counters,
            &commands,
            12,
            -2
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 12,
                selected_item_index: 3,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 12,
                selected_item_index: -1,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn select_bundle_item_counter_tracks_accepted_commands_only() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        assert!(queue_select_bundle_item_command(
            &mut counters,
            &commands,
            12,
            3
        ));
        assert!(!queue_select_bundle_item_command(
            &mut counters,
            &commands,
            12,
            4
        ));

        assert_eq!(counters.select_bundle_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SelectBundleItem(SelectBundleItem {
                slot_id: 12,
                selected_item_index: 3,
            })
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn queues_start_destroy_block_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let hit = CrosshairBlockHit {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            face: ProtocolDirection::West,
            cursor: [0.0, 0.5, 0.5],
            inside: false,
        };

        queue_player_action_command(
            &mut counters,
            &commands,
            PlayerActionKind::StartDestroyBlock,
            hit.pos,
            hit.face,
            3,
        );

        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
                direction: ProtocolDirection::West,
                sequence: 3,
            })
        );
    }

    #[test]
    fn queues_entity_attack_and_interact_commands() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let location = ProtocolVec3d {
            x: 0.25,
            y: 1.0,
            z: -0.5,
        };

        queue_attack_entity_command(&mut counters, &commands, 123);
        queue_interact_entity_command(
            &mut counters,
            &commands,
            123,
            InteractionHand::OffHand,
            location,
            true,
        );

        assert_eq!(counters.attack_entity_commands_queued, 1);
        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(AttackEntity { entity_id: 123 })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntity {
                entity_id: 123,
                hand: InteractionHand::OffHand,
                location,
                using_secondary_action: true,
            })
        );
    }

    #[test]
    fn queues_use_item_on_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();
        let hit = CrosshairBlockHit {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            face: ProtocolDirection::South,
            cursor: [0.25, 0.5, 0.75],
            inside: false,
        };

        queue_use_item_on_command(&mut counters, &commands, hit, 5);

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: ProtocolBlockHitResult {
                    pos: ProtocolBlockPos {
                        x: -5,
                        y: 70,
                        z: 12
                    },
                    direction: ProtocolDirection::South,
                    cursor_x: 0.25,
                    cursor_y: 0.5,
                    cursor_z: 0.75,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 5,
            })
        );
    }

    #[test]
    fn queues_pick_item_from_block_for_crosshair_hit() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_pick_item_from_block_command(
            &mut counters,
            &commands,
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            true,
        );

        assert_eq!(counters.pick_item_from_block_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromBlock(PickItemFromBlock {
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                include_data: true,
            })
        );
    }

    #[test]
    fn queues_pick_item_from_entity() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut counters = NetCounters::default();

        queue_pick_item_from_entity_command(&mut counters, &commands, 123, true);

        assert_eq!(counters.pick_item_from_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromEntity(PickItemFromEntity {
                entity_id: 123,
                include_data: true,
            })
        );
    }
}
