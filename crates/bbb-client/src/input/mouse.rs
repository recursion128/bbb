//! Captured-mouse gameplay routing — the `MouseHandler` decision surface.
//!
//! Three vanilla `MouseHandler` owners intentionally have no equivalent here:
//! `simulateRightClick` is gated on
//! `InputQuirks.SIMULATE_RIGHT_CLICK_WITH_LONG_LEFT_CLICK`, the macOS
//! Ctrl+left-click quirk, false on this client's declared Linux platform;
//! `fillMousePositionDetails` only decorates the `ReportedException` crash
//! categories vanilla wraps around per-screen mouse/render handlers, and BBB
//! routes panics to the one guarded crash report instead of wrapping each
//! handler; `drawDebugMouseInfo` is gated on `SharedConstants.DEBUG_CURSOR_POS`,
//! a dev-only constant that is false in every shipped vanilla client.

use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_world::{
    BlockPos as WorldBlockPos, ItemAttackRange, LevelEventSoundRandomState, LocalInteractionResult,
    LocalInteractionSoundEffect, LocalInteractionSuccess, LocalInteractionSwingSource,
    LocalItemUsePovResult, LocalItemUsePrediction, LocalPlayerPoseState, LocalTargetUsePrediction,
    WorldStore,
};
use bbb_protocol::packets::{
    Direction as ProtocolDirection, InteractionHand, PlayerActionKind, SpectateEntity,
};
use tokio::sync::mpsc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

use crate::audio_runtime::AudioEventSink;
use crate::camera_pose::camera_pose_from_world_at_partial_tick;
use crate::crosshair::{
    crosshair_target_from_camera, item_use_pov_block_hit_from_world,
    local_item_use_pov_result_from_crosshair_hit, CrosshairBlockHit, CrosshairEntityHit,
    CrosshairTarget,
};
use crate::item_models::FirstPersonHandPresentation;
use crate::particle_runtime::local_destroy_hit_particle_state_with_biome_blend_radius;

use super::{
    commands::{
        hotbar_slot_for_scroll, queue_attack_entity_command, queue_interact_entity_command,
        queue_pick_item_from_block_command, queue_pick_item_from_entity_command,
        queue_spectate_entity_command, queue_swing_command, queue_zero_pos_player_action_command,
        select_hotbar_slot, send_reserved_player_action_command, send_reserved_use_item_command,
        send_reserved_use_item_on_command,
    },
    release_active_input, BoundInput, ClientInputState,
};
#[cfg(test)]
use bbb_protocol::entity_types::*;

const USE_ITEM_REPEAT_DELAY_TICKS: u8 = 4;
const SPECTATOR_MOUSE_WHEEL_FLYING_SPEED_STEP: f32 = 0.005;

pub(crate) fn handle_mouse_motion(input: &mut ClientInputState, delta: (f64, f64)) {
    if !input.focused {
        return;
    }
    input.mouse_delta_x += delta.0;
    input.mouse_delta_y += delta.1;
}

#[cfg(test)]
fn handle_mouse_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
) {
    handle_mouse_input_at_partial_tick(input, world, counters, net_commands, button, state, 1.0);
}

#[cfg(test)]
pub(crate) fn handle_mouse_input_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    entity_partial_tick: f32,
) {
    let mut random = LevelEventSoundRandomState::default();
    let mut audio_events = None;
    let mut first_person_hand_presentation = FirstPersonHandPresentation::default();
    handle_mouse_input_at_partial_tick_with_effects_and_presentation(
        input,
        &mut first_person_hand_presentation,
        world,
        counters,
        net_commands,
        button,
        state,
        entity_partial_tick,
        &mut random,
        &mut audio_events,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_mouse_input_at_partial_tick_with_effects_and_presentation(
    input: &mut ClientInputState,
    first_person_hand_presentation: &mut FirstPersonHandPresentation,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    entity_partial_tick: f32,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    if !gameplay_pointer_input_allowed(input, world) {
        return;
    }
    input.set_mouse_button_down(button, matches!(state, ElementState::Pressed));
    handle_gameplay_bound_input(
        input,
        first_person_hand_presentation,
        world,
        counters,
        net_commands,
        BoundInput::Mouse(button),
        state,
        entity_partial_tick,
        level_random,
        audio_events,
    );
}

fn gameplay_pointer_input_allowed(input: &ClientInputState, world: &WorldStore) -> bool {
    input.focused
        && !world.local_player_is_sleeping()
        && !input.sign_editor_is_open()
        && input.death_screen().is_none()
        && input.game_event_screen_runtime().is_none()
        && input.screen.book().is_none()
        && !input.advancements_screen_is_open()
        && input.game_rules_screen().is_none()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_gameplay_bound_input(
    input: &mut ClientInputState,
    first_person_hand_presentation: &mut FirstPersonHandPresentation,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    bound: BoundInput,
    state: ElementState,
    entity_partial_tick: f32,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) -> bool {
    if !gameplay_pointer_input_allowed(input, world) {
        return false;
    }
    let pressed = matches!(state, ElementState::Pressed);
    if pressed
        && input.binding_matches_input("key.spectatorHotbar", bound)
        && world.local_player_is_spectator()
        && (world.local_player().interaction.using_item
            || input.handle_spectator_hotbar_action(world, counters, net_commands))
    {
        return true;
    }
    let matches_attack = input.binding_matches_input("key.attack", bound);
    let matches_use = input.binding_matches_input("key.use", bound);
    let matches_pick = input.binding_matches_input("key.pickItem", bound);
    if !matches_attack && !matches_use && !matches_pick {
        return false;
    }
    let player_pose = world.local_player_pose();
    let camera_target = if pressed {
        crosshair_target_from_camera(
            world,
            camera_pose_from_world_at_partial_tick(world, entity_partial_tick),
        )
    } else {
        None
    };

    if matches_attack {
        let was_down = input.attack_down;
        let next_down = input.toggled_down(input.toggle_attack, was_down, pressed, pressed);
        input.attack_down = next_down;
        if pressed && next_down {
            start_attack_action(
                input,
                world,
                counters,
                net_commands,
                player_pose,
                camera_target,
                level_random,
                audio_events,
            );
        } else if !next_down && (was_down || !pressed) {
            stop_attack_action(input, world, counters, net_commands);
        }
    }

    if matches_use {
        let was_down = input.use_down;
        let next_down = input.toggled_down(input.toggle_use, was_down, pressed, pressed);
        input.use_down = next_down;
        if pressed && next_down {
            input.use_item_held = true;
            start_use_item_with_effects(
                input,
                first_person_hand_presentation,
                world,
                counters,
                net_commands,
                camera_target,
                player_pose,
                level_random,
                audio_events,
            );
        } else if !next_down && (was_down || !pressed) {
            input.use_item_held = false;
            input.use_item_repeat_delay_ticks = 0;
            if world.local_player().interaction.using_item {
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::ReleaseUseItem,
                );
                world.release_local_using_item();
                flush_local_interaction_sound_actions(world, level_random, audio_events);
            }
        }
    }

    if matches_pick && pressed {
        match camera_target {
            Some(CrosshairTarget::Entity(hit)) => {
                queue_pick_item_from_entity_command(
                    counters,
                    net_commands,
                    hit.entity_id,
                    input.control_down(),
                );
            }
            Some(CrosshairTarget::Block(hit)) => {
                queue_pick_item_from_block_command(
                    counters,
                    net_commands,
                    hit.pos,
                    input.control_down(),
                );
            }
            None => {}
        }
    }

    true
}

#[allow(clippy::too_many_arguments)]
fn start_attack_action(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    player_pose: Option<LocalPlayerPoseState>,
    camera_target: Option<CrosshairTarget>,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    if local_player_attack_is_blocked(input, world) {
        return;
    }
    if local_player_hands_are_busy(input, world) {
        return;
    }
    if world.local_player_is_spectator() {
        if let Some(CrosshairTarget::Entity(hit)) = camera_target {
            queue_spectate_entity_command(
                counters,
                net_commands,
                SpectateEntity {
                    entity_id: hit.entity_id,
                },
            );
        }
        return;
    }
    if !world.local_selected_main_hand_is_enabled()
        || world.local_player_cannot_attack_with_selected_item()
    {
        return;
    }
    if world.local_selected_main_hand_has_piercing_weapon() {
        queue_zero_pos_player_action_command(counters, net_commands, PlayerActionKind::Stab);
        world.reset_only_local_player_attack_strength();
        world.queue_local_piercing_weapon_use_sound();
        flush_local_interaction_sound_actions(world, level_random, audio_events);
        queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
        return;
    }
    input.destroy_block_held = true;
    match camera_target {
        Some(CrosshairTarget::Entity(hit)) => {
            if selected_attack_range_allows_entity_hit(world, player_pose, hit) {
                queue_attack_entity_command(
                    counters,
                    net_commands,
                    hit.entity_id,
                    world.local_player().selected_hotbar_slot,
                );
                let was_sprinting = world.local_player_is_sprinting();
                world.apply_local_player_attack_prediction(hit.entity_id, was_sprinting);
                world.reset_local_player_attack_strength();
            }
        }
        Some(CrosshairTarget::Block(hit)) => {
            if block_target_within_world_border(world, hit.pos) {
                start_destroy_block(input, counters, world, net_commands, hit);
            }
        }
        None => {
            if !world.local_player_is_creative() {
                input.attack_miss_time_ticks = 10;
            }
            world.reset_local_player_attack_strength();
        }
    }
    queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
}

fn stop_attack_action(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    input.attack_miss_time_ticks = 0;
    input.destroy_block_held = false;
    if world.local_selected_main_hand_has_piercing_weapon() {
        return;
    }
    abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
}

#[cfg(test)]
pub(crate) fn advance_using_item_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_partial_tick: f32,
    use_ticks: u32,
) {
    let mut random = LevelEventSoundRandomState::default();
    let mut audio_events = None;
    let mut first_person_hand_presentation = FirstPersonHandPresentation::default();
    advance_using_item_at_partial_tick_with_effects_and_presentation(
        input,
        &mut first_person_hand_presentation,
        world,
        counters,
        net_commands,
        entity_partial_tick,
        use_ticks,
        &mut random,
        &mut audio_events,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_using_item_at_partial_tick_with_effects_and_presentation(
    input: &mut ClientInputState,
    first_person_hand_presentation: &mut FirstPersonHandPresentation,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_partial_tick: f32,
    use_ticks: u32,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    if !input.focused
        || input.sign_editor_is_open()
        || input.death_screen().is_some()
        || input.game_event_screen_runtime().is_some()
        || input.screen.book().is_some()
        || !input.use_item_held
        || use_ticks == 0
    {
        return;
    }

    let mut ticks_remaining = use_ticks;
    loop {
        if world.local_player().interaction.using_item
            || world.local_player().interaction.destroying_block.is_some()
        {
            return;
        }

        let delay_ticks = u32::from(input.use_item_repeat_delay_ticks);
        if delay_ticks > ticks_remaining {
            input.use_item_repeat_delay_ticks = (delay_ticks - ticks_remaining) as u8;
            return;
        }
        ticks_remaining -= delay_ticks;
        input.use_item_repeat_delay_ticks = 0;

        let camera_target = crosshair_target_from_camera(
            world,
            camera_pose_from_world_at_partial_tick(world, entity_partial_tick),
        );
        let player_pose = world.local_player_pose();
        if !start_use_item_with_effects(
            input,
            first_person_hand_presentation,
            world,
            counters,
            net_commands,
            camera_target,
            player_pose,
            level_random,
            audio_events,
        ) || ticks_remaining == 0
        {
            return;
        }
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_mouse_input_at_partial_tick_with_effects(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    entity_partial_tick: f32,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    let mut first_person_hand_presentation = FirstPersonHandPresentation::default();
    handle_mouse_input_at_partial_tick_with_effects_and_presentation(
        input,
        &mut first_person_hand_presentation,
        world,
        counters,
        net_commands,
        button,
        state,
        entity_partial_tick,
        level_random,
        audio_events,
    );
}

pub(crate) fn advance_local_using_item_ticks_with_effects(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    use_ticks: u32,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    let actions = world.advance_local_using_item_ticks_with_actions(use_ticks);
    flush_local_interaction_sound_actions(world, level_random, audio_events);
    for _ in 0..actions.bundle_drops {
        apply_local_interaction_sound_action(
            world,
            bbb_client_world::LocalInteractionSoundAction::BundleRemoveOne,
            level_random,
            audio_events,
        );
        queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
    }
}

#[cfg(test)]
pub(super) fn start_use_item(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    camera_target: Option<CrosshairTarget>,
    player_pose: Option<bbb_client_world::LocalPlayerPoseState>,
) -> bool {
    let mut random = LevelEventSoundRandomState::default();
    let mut audio_events = None;
    let mut first_person_hand_presentation = FirstPersonHandPresentation::default();
    start_use_item_with_effects(
        input,
        &mut first_person_hand_presentation,
        world,
        counters,
        net_commands,
        camera_target,
        player_pose,
        &mut random,
        &mut audio_events,
    )
}

#[allow(clippy::too_many_arguments)]
fn start_use_item_with_effects(
    input: &mut ClientInputState,
    first_person_hand_presentation: &mut FirstPersonHandPresentation,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    camera_target: Option<CrosshairTarget>,
    player_pose: Option<bbb_client_world::LocalPlayerPoseState>,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) -> bool {
    if world.local_player().interaction.destroying_block.is_some() {
        return false;
    }
    input.use_item_repeat_delay_ticks = USE_ITEM_REPEAT_DELAY_TICKS;
    if world.local_player().interaction.using_item || local_player_hands_are_busy(input, world) {
        return false;
    }
    let target_in_world_border = match camera_target {
        Some(CrosshairTarget::Entity(hit)) => entity_target_within_world_border(world, hit),
        Some(CrosshairTarget::Block(hit)) => block_target_within_world_border(world, hit.pos),
        None => true,
    };
    if !target_in_world_border {
        return false;
    }

    if let Some(CrosshairTarget::Entity(hit)) = camera_target {
        if world.try_open_minecart_command_block_editor(hit.entity_id) {
            input.apply_screen_requests(world, counters, net_commands);
        }
    }

    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        if !world.local_item_in_hand_is_enabled(hand) {
            return true;
        }

        let pre_use_stack = world.local_item_in_hand(hand).cloned();
        let target_prediction = match camera_target {
            Some(CrosshairTarget::Entity(hit)) => {
                let prediction =
                    world.predict_local_entity_interaction(hit.entity_id, hand, input.sneak);
                queue_interact_entity_command(
                    counters,
                    net_commands,
                    world.local_player().selected_hotbar_slot,
                    hit.entity_id,
                    hand,
                    hit.relative_location,
                    input.sneak,
                );
                Some(prediction)
            }
            Some(CrosshairTarget::Block(hit)) => {
                let Some(tx) = net_commands else {
                    return true;
                };
                let Ok(permit) = tx.try_reserve() else {
                    return true;
                };
                let sequence = world.next_local_prediction_sequence();
                let both_hands_empty = world
                    .local_item_in_hand(InteractionHand::MainHand)
                    .is_none_or(|stack| stack.count <= 0)
                    && world
                        .local_item_in_hand(InteractionHand::OffHand)
                        .is_none_or(|stack| stack.count <= 0);
                // Vanilla `MultiPlayerGameMode.useItemOn`: sneaking while holding
                // any item suppresses block-use actions (including opening command,
                // structure, jigsaw, and test block editors) so the player can
                // sneak-place blocks against them.
                if hand == InteractionHand::MainHand && (!input.sneak || both_hands_empty) {
                    if world.try_open_command_block_editor(hit.pos) {
                        input.apply_screen_requests(world, counters, net_commands);
                    }
                    if let Some(editor) = world.structure_block_editor_at(hit.pos) {
                        input.open_structure_block_editor(world, counters, net_commands, editor);
                    }
                    if let Some(editor) = world.jigsaw_block_editor_at(hit.pos) {
                        input.open_jigsaw_block_editor(world, counters, net_commands, editor);
                    }
                    if let Some(editor) = world.test_block_editor_at(hit.pos) {
                        input.open_test_block_editor(world, counters, net_commands, editor);
                    }
                    if let Some(editor) = world.test_instance_block_editor_at(hit.pos) {
                        input.open_test_instance_block_editor(
                            world,
                            counters,
                            net_commands,
                            editor,
                        );
                    }
                }
                let prediction = world.predict_local_block_use_with_hit_and_random(
                    hit.pos,
                    hit.face,
                    hit.cursor,
                    hit.inside,
                    hand,
                    input.sneak,
                    sequence,
                    level_random,
                );
                send_reserved_use_item_on_command(
                    counters,
                    permit,
                    world.local_player().selected_hotbar_slot,
                    hand,
                    hit,
                    sequence,
                );
                Some(prediction)
            }
            None => None,
        };
        for action in world.take_pending_local_interaction_particle_actions() {
            input.queue_local_interaction_particle_action(action);
        }
        if let Some(prediction) = target_prediction {
            match prediction {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success)) => {
                    let item_used = matches!(camera_target, Some(CrosshairTarget::Block(_)))
                        && block_success_uses_first_person_item(
                            pre_use_stack.as_ref(),
                            world,
                            hand,
                            success,
                        );
                    apply_local_interaction_success(
                        input,
                        first_person_hand_presentation,
                        world,
                        counters,
                        net_commands,
                        hand,
                        success,
                        item_used,
                        level_random,
                        audio_events,
                    );
                    return true;
                }
                LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
                    if matches!(camera_target, Some(CrosshairTarget::Block(_))) =>
                {
                    return true;
                }
                LocalTargetUsePrediction::Result(
                    LocalInteractionResult::Pass | LocalInteractionResult::Fail,
                ) => {}
                LocalTargetUsePrediction::UnsupportedItemUseOn(_) if matches!(camera_target, Some(CrosshairTarget::Block(hit)) if !world.content_registry().is_vanilla_only() && world.block_view(hit.pos).is_some_and(|block| world.content_registry().block_by_state_id(block.block_state_id).is_some())) =>
                    {}
                LocalTargetUsePrediction::UnknownBlockType(Some(state_id))
                    if !world.content_registry().is_vanilla_only()
                        && world
                            .content_registry()
                            .block_by_state_id(state_id)
                            .is_some() => {}
                LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id))
                    if !world.content_registry().is_vanilla_only()
                        && world
                            .content_registry()
                            .entity_by_id(entity_type_id)
                            .is_some() => {}
                LocalTargetUsePrediction::UnknownItemType(Some(item_id))
                    if !world.content_registry().is_vanilla_only()
                        && world.content_registry().item_by_id(item_id).is_some() => {}
                LocalTargetUsePrediction::Result(
                    LocalInteractionResult::TryEmptyHandInteraction,
                )
                | LocalTargetUsePrediction::UnsupportedBlockUseItemOn(_)
                | LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(_)
                | LocalTargetUsePrediction::UnsupportedItemUseOn(_)
                | LocalTargetUsePrediction::UnsupportedEntityInteract(_)
                | LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(_)
                | LocalTargetUsePrediction::UnsupportedItemStackInteractLivingEntityComponent(_)
                | LocalTargetUsePrediction::UnknownBlockType(_)
                | LocalTargetUsePrediction::UnknownEntityType(_)
                | LocalTargetUsePrediction::UnknownItemType(_) => return true,
            }
        }

        if world.local_item_in_hand(hand).is_none() || world.local_player_is_spectator() {
            continue;
        }
        let Some(pose) = player_pose else {
            return true;
        };
        let pov = world.local_item_use_pov_query(hand).map(|query| {
            item_use_pov_block_hit_from_world(world, pose, query).map_or(
                LocalItemUsePovResult::Miss,
                local_item_use_pov_result_from_crosshair_hit,
            )
        });
        let Some(tx) = net_commands else {
            return true;
        };
        let Ok(permit) = tx.try_reserve() else {
            return true;
        };
        let sequence = world.next_local_prediction_sequence();
        let prediction =
            world.predict_local_item_use_with_pov_and_sequence(hand, pov, sequence, input.sneak);
        send_reserved_use_item_command(
            counters,
            permit,
            world.local_player().selected_hotbar_slot,
            hand,
            pose.y_rot,
            pose.x_rot,
            sequence,
        );
        let opened_writable_book =
            input.open_writable_book_editor(world, counters, net_commands, hand);
        match prediction {
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(success)) => {
                apply_local_interaction_success(
                    input,
                    first_person_hand_presentation,
                    world,
                    counters,
                    net_commands,
                    hand,
                    success,
                    true,
                    level_random,
                    audio_events,
                );
                if opened_writable_book {
                    release_active_input(input, world, counters, net_commands);
                }
                return true;
            }
            LocalItemUsePrediction::Result(
                LocalInteractionResult::Pass | LocalInteractionResult::Fail,
            ) => {}
            LocalItemUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
            | LocalItemUsePrediction::UnsupportedOverride(_)
            | LocalItemUsePrediction::UnknownItemType(_)
            | LocalItemUsePrediction::MissingComponentValue(_)
            | LocalItemUsePrediction::MissingRegistryTag(_) => return true,
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn apply_local_interaction_success(
    input: &mut ClientInputState,
    first_person_hand_presentation: &mut FirstPersonHandPresentation,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hand: InteractionHand,
    success: LocalInteractionSuccess,
    item_used: bool,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    if let Some(action) = success.sound_action {
        apply_local_interaction_sound_action(world, action, level_random, audio_events);
    }
    if let Some(extra_swing) = success.extra_swing {
        queue_swing_command(world, counters, net_commands, extra_swing);
    }
    if let Some(action) = success.particle_action {
        input.queue_local_interaction_particle_action(action);
    }
    world.set_local_using_item_with_hand(success.starts_using_item, hand);
    if success.swing_source == LocalInteractionSwingSource::Client {
        queue_swing_command(world, counters, net_commands, hand);
    }
    if item_used {
        first_person_hand_presentation.item_used(hand);
    }
}

fn block_success_uses_first_person_item(
    pre_use_stack: Option<&bbb_protocol::packets::ItemStackSummary>,
    world: &WorldStore,
    hand: InteractionHand,
    success: LocalInteractionSuccess,
) -> bool {
    if success.swing_source != LocalInteractionSwingSource::Client {
        return false;
    }
    let Some(pre_use_stack) = pre_use_stack.filter(|stack| stack.count > 0) else {
        return false;
    };
    let current_count = world
        .local_item_in_hand(hand)
        .map_or(0, |stack| stack.count);
    current_count != pre_use_stack.count || local_player_instabuild(world)
}

pub(crate) fn flush_local_interaction_sound_actions(
    world: &mut WorldStore,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    for action in world.take_pending_local_interaction_sound_actions() {
        apply_local_interaction_sound_action(world, action, level_random, audio_events);
    }
}

fn apply_local_interaction_sound_action(
    world: &mut WorldStore,
    action: bbb_client_world::LocalInteractionSoundAction,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    let Some(effect) = world.apply_local_interaction_sound_action(action, level_random) else {
        return;
    };
    let Some(audio_events) = audio_events.as_deref_mut() else {
        return;
    };
    match effect {
        LocalInteractionSoundEffect::Local(state) => {
            audio_events.play_local_sound(&state);
        }
        LocalInteractionSoundEffect::Positioned(state) => {
            audio_events.play_positioned_sound(&state);
        }
        LocalInteractionSoundEffect::Entity { state, position } => {
            audio_events.play_entity_sound(
                &state,
                position.map(|position| [position.x, position.y, position.z]),
            );
        }
    }
}

fn entity_target_within_world_border(world: &WorldStore, hit: CrosshairEntityHit) -> bool {
    let (x, y, z) = if let Some(entity) = world.probe_entity(hit.entity_id) {
        (entity.position.x, entity.position.y, entity.position.z)
    } else {
        (hit.location.x, hit.location.y, hit.location.z)
    };
    world.world_border().contains_block_pos(WorldBlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    })
}

fn block_target_within_world_border(world: &WorldStore, pos: WorldBlockPos) -> bool {
    world.world_border().contains_block_pos(pos)
}

fn selected_attack_range_allows_entity_hit(
    world: &WorldStore,
    player_pose: Option<LocalPlayerPoseState>,
    hit: CrosshairEntityHit,
) -> bool {
    let Some(attack_range) = world.local_selected_main_hand_attack_range() else {
        return true;
    };
    let Some(player_pose) = player_pose else {
        return false;
    };
    attack_range_contains_hit(
        attack_range,
        bbb_protocol::packets::Vec3d {
            y: player_pose.position.y + world.local_player_eye_height(player_pose),
            ..player_pose.position
        },
        hit.location,
        world,
    )
}

fn attack_range_contains_hit(
    attack_range: ItemAttackRange,
    eye: bbb_protocol::packets::Vec3d,
    hit: bbb_protocol::packets::Vec3d,
    world: &WorldStore,
) -> bool {
    let distance =
        ((hit.x - eye.x).powi(2) + (hit.y - eye.y).powi(2) + (hit.z - eye.z).powi(2)).sqrt();
    let (min_reach, max_reach) = if local_player_instabuild(world) {
        (
            attack_range.min_creative_reach,
            attack_range.max_creative_reach,
        )
    } else {
        (attack_range.min_reach, attack_range.max_reach)
    };
    distance >= f64::from(min_reach - attack_range.hitbox_margin)
        && distance <= f64::from(max_reach + attack_range.hitbox_margin)
}

#[cfg(test)]
pub(crate) fn advance_destroying_block_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    entity_partial_tick: f32,
    destroy_ticks: u32,
) {
    advance_destroying_block_at_partial_tick_with_biome_blend_radius(
        input,
        world,
        counters,
        net_commands,
        audio_events,
        entity_partial_tick,
        destroy_ticks,
        2,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_destroying_block_at_partial_tick_with_biome_blend_radius(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    entity_partial_tick: f32,
    destroy_ticks: u32,
    biome_blend_radius: u32,
) {
    let mut audio_events = audio_events;
    if !input.destroy_block_held && world.local_player().interaction.destroying_block.is_some() {
        abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        return;
    }
    if !input.focused
        || input.sign_editor_is_open()
        || input.death_screen().is_some()
        || input.game_event_screen_runtime().is_some()
        || input.screen.book().is_some()
        || !input.destroy_block_held
    {
        return;
    }
    if local_player_attack_is_blocked(input, world) {
        return;
    }
    if world.local_selected_main_hand_has_piercing_weapon() {
        return;
    }
    for _ in 0..destroy_ticks {
        let camera_target = crosshair_target_from_camera(
            world,
            camera_pose_from_world_at_partial_tick(world, entity_partial_tick),
        );
        let Some(CrosshairTarget::Block(hit)) = camera_target else {
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
            continue;
        };
        if continue_destroy_block_tick(input, counters, world, net_commands, &mut audio_events, hit)
        {
            if let Some(state) = local_destroy_hit_particle_state_with_biome_blend_radius(
                world,
                hit.pos,
                hit.face,
                biome_blend_radius,
            ) {
                input.queue_local_destroy_hit_particles(state);
            }
            queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
        }
    }
}

fn continue_destroy_block_tick(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    hit: CrosshairBlockHit,
) -> bool {
    if world.tick_local_destroy_delay() {
        return true;
    }
    if world.local_player().interaction.destroying_block == Some(hit.pos)
        && world.local_destroying_block_matches_current_item()
    {
        let completion_permit = if world.local_destroying_block_will_finish_next_tick() {
            let Some(tx) = net_commands else {
                return false;
            };
            let Ok(permit) = tx.try_reserve() else {
                return false;
            };
            Some(permit)
        } else {
            None
        };
        world.update_local_destroying_block_face(hit.face);
        maybe_play_destroy_hit_sound(audio_events, world, hit.pos);
        if let Some(finished) = world.advance_local_destroying_block_tick() {
            let permit =
                completion_permit.expect("finishing destroy reserved its player-action command");
            send_reserved_player_action_command(
                counters,
                permit,
                PlayerActionKind::StopDestroyBlock,
                finished.pos,
                finished.face,
                finished.sequence,
            );
            return true;
        }
        return world.local_player().interaction.destroying_block == Some(hit.pos);
    }

    retarget_destroy_block(input, counters, world, net_commands, hit)
}

fn retarget_destroy_block(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
) -> bool {
    if local_player_block_action_is_restricted(world, hit.pos)
        || !block_target_within_world_border(world, hit.pos)
    {
        return false;
    }
    let command_count = if world.local_player().interaction.destroying_block.is_some() {
        2
    } else {
        1
    };
    let Some(tx) = net_commands else {
        return false;
    };
    let Ok(mut permits) = tx.try_reserve_many(command_count) else {
        return false;
    };
    if let Some(old_pos) = world.take_local_destroying_block() {
        send_reserved_player_action_command(
            counters,
            permits.next().expect("retarget reserved its abort command"),
            PlayerActionKind::AbortDestroyBlock,
            old_pos,
            hit.face,
            0,
        );
    }
    apply_start_destroy_block(
        input,
        counters,
        world,
        permits.next().expect("retarget reserved its start command"),
        hit,
    );
    true
}

fn maybe_play_destroy_hit_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    world: &WorldStore,
    pos: WorldBlockPos,
) {
    if !world
        .local_player()
        .interaction
        .destroying_block_ticks
        .is_multiple_of(4)
    {
        return;
    }
    let Some(sound) = world.local_block_hit_sound(pos) else {
        return;
    };
    if let Some(audio_events) = audio_events.as_mut() {
        audio_events.play_positioned_sound(&sound);
    }
}

fn start_destroy_block(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
) -> bool {
    if local_player_block_action_is_restricted(world, hit.pos)
        || !block_target_within_world_border(world, hit.pos)
    {
        return false;
    }

    if !local_player_instabuild(world)
        && world.local_player().interaction.destroying_block.is_some()
    {
        if world.local_player().interaction.destroying_block == Some(hit.pos)
            && world.local_destroying_block_matches_current_item()
        {
            return true;
        }
        return retarget_destroy_block(input, counters, world, net_commands, hit);
    }

    let Some(tx) = net_commands else {
        return false;
    };
    let Ok(permit) = tx.try_reserve() else {
        return false;
    };
    apply_start_destroy_block(input, counters, world, permit, hit);
    true
}

fn apply_start_destroy_block(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    permit: mpsc::Permit<'_, NetCommand>,
    hit: CrosshairBlockHit,
) {
    let sequence = world.next_local_prediction_sequence();
    if local_player_instabuild(world) {
        world.tutorial_on_destroy_block(hit.pos, 1_000_000);
        if world.local_selected_main_hand_can_destroy_block()
            && world.local_destroy_block_gamemaster_gate_allows(hit.pos)
        {
            world.predict_local_destroy_block(hit.pos, sequence);
        }
        world.set_local_destroy_delay_ticks(5);
    } else {
        if let Some(effect) = world.apply_local_block_attack(hit.pos, sequence) {
            input.queue_local_block_attack_effect(effect);
        }
        world.tutorial_on_destroy_block(hit.pos, 0);
        if world.local_destroy_block_is_immediate(hit.pos) {
            if world.local_selected_main_hand_can_destroy_block()
                && world.local_destroy_block_gamemaster_gate_allows(hit.pos)
            {
                world.predict_local_destroy_block(hit.pos, sequence);
            }
        } else {
            world.set_local_destroying_block_hit(hit.pos, hit.face);
        }
    }
    send_reserved_player_action_command(
        counters,
        permit,
        PlayerActionKind::StartDestroyBlock,
        hit.pos,
        hit.face,
        sequence,
    );
}

fn local_player_block_action_is_restricted(world: &WorldStore, pos: WorldBlockPos) -> bool {
    world.local_player_is_adventure()
        && !world.local_selected_main_hand_can_break_block_in_adventure_mode(pos)
}

pub(super) fn abort_destroy_block(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    direction: ProtocolDirection,
) -> bool {
    let Some(pos) = world.local_player().interaction.destroying_block else {
        return true;
    };
    let Some(tx) = net_commands else {
        return false;
    };
    let Ok(permit) = tx.try_reserve() else {
        return false;
    };
    world.tutorial_on_destroy_block(pos, -1);
    let cleared = world
        .take_local_destroying_block()
        .expect("reserved destroy abort still owns its target");
    world.reset_only_local_player_attack_strength();
    send_reserved_player_action_command(
        counters,
        permit,
        PlayerActionKind::AbortDestroyBlock,
        cleared,
        direction,
        0,
    );
    true
}

fn local_player_instabuild(world: &WorldStore) -> bool {
    world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild)
}

fn local_player_attack_is_blocked(input: &ClientInputState, world: &WorldStore) -> bool {
    world.local_player().interaction.using_item || input.attack_miss_time_ticks > 0
}

pub(crate) fn local_player_hands_are_busy(input: &ClientInputState, world: &WorldStore) -> bool {
    world
        .local_player_first_passenger_root_boat_vehicle_id()
        .is_some()
        && (input.left || input.right || input.forward || input.backward)
}

pub(crate) fn handle_mouse_wheel(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    delta: MouseScrollDelta,
) {
    if !input.focused
        || input.sign_editor_is_open()
        || input.screen.book().is_some()
        || input.advancements_screen_is_open()
        || input.game_rules_screen().is_some()
    {
        return;
    }
    let Some(wheel) = wheel_steps_from_scroll(input, delta) else {
        return;
    };
    if input.adjust_zoom_wheel(wheel.primary()) {
        return;
    }
    if world.local_player_is_spectator() {
        if input.handle_spectator_menu_scroll(world, -wheel.primary()) {
            return;
        }
        if wheel.y != 0 {
            world.adjust_local_flying_speed(
                wheel.y as f32 * SPECTATOR_MOUSE_WHEEL_FLYING_SPEED_STEP,
            );
        }
        return;
    }
    let current_slot = world.local_player().selected_hotbar_slot;
    if let Some(slot) = hotbar_slot_for_scroll(wheel.primary(), current_slot) {
        select_hotbar_slot(counters, world, net_commands, slot);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WheelSteps {
    x: i32,
    y: i32,
}

impl WheelSteps {
    fn primary(self) -> i32 {
        if self.y == 0 {
            -self.x
        } else {
            self.y
        }
    }
}

fn wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: MouseScrollDelta,
) -> Option<WheelSteps> {
    let (x, y) = crate::input::scroll_delta_lines(delta);

    if input.scroll_accumulated_x != 0.0
        && scroll_signum(x) != scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && scroll_signum(y) != scroll_signum(input.scroll_accumulated_y)
    {
        input.scroll_accumulated_y = 0.0;
    }

    input.scroll_accumulated_x += x;
    input.scroll_accumulated_y += y;
    let wheel_x = input.scroll_accumulated_x as i32;
    let wheel_y = input.scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.scroll_accumulated_x -= f64::from(wheel_x);
    input.scroll_accumulated_y -= f64::from(wheel_y);
    Some(WheelSteps {
        x: wheel_x,
        y: wheel_y,
    })
}

fn scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
