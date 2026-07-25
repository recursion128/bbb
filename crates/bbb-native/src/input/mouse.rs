use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    Direction as ProtocolDirection, InteractionHand, PlayerActionKind, SpectateEntity,
};
use bbb_world::{
    BlockPos as WorldBlockPos, ItemAttackRange, LevelEventSoundRandomState,
    LocalDestroyBlockFinished, LocalInteractionResult, LocalInteractionSoundEffect,
    LocalInteractionSuccess, LocalInteractionSwingSource, LocalItemUsePrediction,
    LocalPlayerPoseState, LocalTargetUsePrediction, WorldStore,
};
use tokio::sync::mpsc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

use crate::audio_runtime::AudioEventSink;
use crate::camera_pose::camera_pose_from_world;
use crate::crosshair::{
    crosshair_target_from_camera_at_partial_tick, CrosshairBlockHit, CrosshairEntityHit,
    CrosshairTarget,
};
use crate::particle_runtime::local_destroy_hit_particle_state;

use super::{
    commands::{
        hotbar_slot_for_scroll, queue_attack_entity_command, queue_interact_entity_command,
        queue_pick_item_from_block_command, queue_pick_item_from_entity_command,
        queue_player_action_command, queue_spectate_entity_command, queue_swing_command,
        queue_use_item_command, queue_use_item_on_command, queue_zero_pos_player_action_command,
        select_hotbar_slot,
    },
    release_active_input, ClientInputState,
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
    handle_mouse_input_at_partial_tick_with_effects(
        input,
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
    if !input.focused
        || input.sign_editor_is_active_or_pending(world)
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
        || world.game_rules_screen().is_some()
    {
        return;
    }
    input.set_mouse_button_down(button, matches!(state, ElementState::Pressed));
    if matches!(
        (button, state),
        (MouseButton::Middle, ElementState::Pressed)
    ) && world.local_player_is_spectator()
    {
        if world.local_player().interaction.using_item
            || input.handle_spectator_hotbar_action(world, counters, net_commands)
        {
            return;
        }
    }
    let player_pose = world.local_player_pose();
    let camera_target = match (button, state) {
        (MouseButton::Left | MouseButton::Right | MouseButton::Middle, ElementState::Pressed) => {
            crosshair_target_from_camera_at_partial_tick(
                world,
                camera_pose_from_world(world),
                entity_partial_tick,
            )
        }
        _ => None,
    };
    match (button, state) {
        (MouseButton::Left, ElementState::Pressed) => {
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
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::Stab,
                );
                world.reset_only_local_player_attack_strength();
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
        (MouseButton::Left, ElementState::Released) => {
            input.attack_miss_time_ticks = 0;
            input.destroy_block_held = false;
            if world.local_selected_main_hand_has_piercing_weapon() {
                return;
            }
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        }
        (MouseButton::Right, ElementState::Pressed) => {
            input.use_item_held = true;
            start_use_item_with_effects(
                input,
                world,
                counters,
                net_commands,
                camera_target,
                player_pose,
                level_random,
                audio_events,
            );
        }
        (MouseButton::Right, ElementState::Released) => {
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
        (MouseButton::Middle, ElementState::Pressed) => match camera_target {
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
        },
        _ => {}
    }
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
    advance_using_item_at_partial_tick_with_effects(
        input,
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
pub(crate) fn advance_using_item_at_partial_tick_with_effects(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_partial_tick: f32,
    use_ticks: u32,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    if !input.focused
        || input.sign_editor_is_active_or_pending(world)
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.current_book().is_some()
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

        let camera_target = crosshair_target_from_camera_at_partial_tick(
            world,
            camera_pose_from_world(world),
            entity_partial_tick,
        );
        let player_pose = world.local_player_pose();
        if !start_use_item_with_effects(
            input,
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
            bbb_world::LocalInteractionSoundAction::BundleRemoveOne,
            level_random,
            audio_events,
        );
        queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
        apply_local_interaction_sound_action(
            world,
            bbb_world::LocalInteractionSoundAction::SuppressedBundleDropContents,
            level_random,
            audio_events,
        );
    }
}

#[cfg(test)]
pub(super) fn start_use_item(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    camera_target: Option<CrosshairTarget>,
    player_pose: Option<bbb_world::LocalPlayerPoseState>,
) -> bool {
    let mut random = LevelEventSoundRandomState::default();
    let mut audio_events = None;
    start_use_item_with_effects(
        input,
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
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    camera_target: Option<CrosshairTarget>,
    player_pose: Option<bbb_world::LocalPlayerPoseState>,
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
    match camera_target {
        Some(CrosshairTarget::Entity(hit)) => {
            if !entity_target_within_world_border(world, hit) {
                return false;
            }
        }
        Some(CrosshairTarget::Block(hit)) => {
            if !block_target_within_world_border(world, hit.pos) {
                return false;
            }
        }
        None => {}
    }

    match camera_target {
        Some(CrosshairTarget::Entity(hit)) => {
            if world.try_open_minecart_command_block_editor(hit.entity_id) {
                input.sync_minecart_command_block_editor(world, counters, net_commands);
            }
        }
        Some(CrosshairTarget::Block(hit)) => {
            if world.try_open_command_block_editor(hit.pos) {
                input.sync_command_block_editor(world, counters, net_commands);
            }
            if world.try_open_structure_block_editor(hit.pos) {
                input.sync_structure_block_editor(world, counters, net_commands);
            }
            if world.try_open_jigsaw_block_editor(hit.pos) {
                input.sync_jigsaw_block_editor(world, counters, net_commands);
            }
            if world.try_open_test_block_editor(hit.pos) {
                input.sync_test_block_editor(world, counters, net_commands);
            }
            if world.try_open_test_instance_block_editor(hit.pos) {
                input.sync_test_instance_block_editor(world, counters, net_commands);
            }
        }
        None => {}
    }

    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        if !world.local_item_in_hand_is_enabled(hand) {
            return true;
        }

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
                let sequence = world.next_local_prediction_sequence();
                let prediction =
                    world.predict_local_block_use(hit.pos, hit.face, hand, input.sneak, sequence);
                queue_use_item_on_command(
                    counters,
                    net_commands,
                    world.local_player().selected_hotbar_slot,
                    hand,
                    hit,
                    sequence,
                );
                Some(prediction)
            }
            None => None,
        };
        if let Some(prediction) = target_prediction {
            match prediction {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success)) => {
                    apply_local_interaction_success(
                        world,
                        counters,
                        net_commands,
                        hand,
                        success,
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
        let prediction = world.predict_local_item_use(hand);
        let sequence = world.next_local_prediction_sequence();
        if !queue_use_item_command(
            counters,
            net_commands,
            world.local_player().selected_hotbar_slot,
            hand,
            pose.y_rot,
            pose.x_rot,
            sequence,
        ) {
            return true;
        }
        let opened_writable_book =
            world.open_writable_book_editor(hand, input.client_text_filtering_enabled());
        match prediction {
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(success)) => {
                apply_local_interaction_success(
                    world,
                    counters,
                    net_commands,
                    hand,
                    success,
                    level_random,
                    audio_events,
                );
                if opened_writable_book {
                    input.sync_book_screen(world);
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

fn apply_local_interaction_success(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hand: InteractionHand,
    success: LocalInteractionSuccess,
    level_random: &mut LevelEventSoundRandomState,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    if let Some(action) = success.sound_action {
        apply_local_interaction_sound_action(world, action, level_random, audio_events);
    }
    if let Some(extra_swing) = success.extra_swing {
        queue_swing_command(world, counters, net_commands, extra_swing);
    }
    world.set_local_using_item_with_hand(success.starts_using_item, hand);
    if success.swing_source == LocalInteractionSwingSource::Client {
        queue_swing_command(world, counters, net_commands, hand);
    }
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
    action: bbb_world::LocalInteractionSoundAction,
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
        local_player_eye(player_pose),
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

fn local_player_eye(player_pose: LocalPlayerPoseState) -> bbb_protocol::packets::Vec3d {
    bbb_protocol::packets::Vec3d {
        x: player_pose.position.x,
        y: player_pose.position.y + player_pose.eye_height(),
        z: player_pose.position.z,
    }
}

pub(crate) fn advance_destroying_block_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    entity_partial_tick: f32,
    destroy_ticks: u32,
) {
    let mut audio_events = audio_events;
    if !input.focused
        || input.sign_editor_is_active_or_pending(world)
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.current_book().is_some()
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
    if destroy_ticks == 0 {
        let camera_target = crosshair_target_from_camera_at_partial_tick(
            world,
            camera_pose_from_world(world),
            entity_partial_tick,
        );
        if let Some(CrosshairTarget::Block(hit)) = camera_target {
            if block_target_within_world_border(world, hit.pos)
                && (world.local_player().interaction.destroying_block != Some(hit.pos)
                    || !world.local_destroying_block_matches_current_item())
            {
                retarget_destroy_block(input, counters, world, net_commands, hit);
            }
        } else {
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        }
        return;
    }

    for _ in 0..destroy_ticks {
        let camera_target = crosshair_target_from_camera_at_partial_tick(
            world,
            camera_pose_from_world(world),
            entity_partial_tick,
        );
        let Some(CrosshairTarget::Block(hit)) = camera_target else {
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
            continue;
        };
        if !block_target_within_world_border(world, hit.pos) {
            continue;
        }
        if continue_destroy_block_tick(input, counters, world, net_commands, &mut audio_events, hit)
        {
            if let Some(state) = local_destroy_hit_particle_state(world, hit.pos, hit.face) {
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
        world.update_local_destroying_block_face(hit.face);
        maybe_play_destroy_hit_sound(audio_events, world, hit.pos);
        if let Some(finished) = world.advance_local_destroying_block_tick() {
            stop_destroy_block(counters, net_commands, finished);
        }
        return true;
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
    if local_player_block_action_is_restricted(world, hit.pos) {
        return false;
    }
    if let Some(old_pos) = world.take_local_destroying_block() {
        queue_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::AbortDestroyBlock,
            old_pos,
            hit.face,
            0,
        );
    }
    start_destroy_block(input, counters, world, net_commands, hit)
}

fn maybe_play_destroy_hit_sound(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    world: &WorldStore,
    pos: WorldBlockPos,
) {
    if world.local_player().interaction.destroying_block_ticks % 4 != 0 {
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

    let sequence = world.next_local_prediction_sequence();
    if local_player_instabuild(world) {
        if world.local_selected_main_hand_can_destroy_block_in_creative() {
            world.predict_local_destroy_block(hit.pos, sequence);
        }
        world.set_local_destroy_delay_ticks(5);
    } else {
        if let Some(effect) = world.apply_local_block_attack(hit.pos, sequence) {
            input.queue_local_block_attack_effect(effect);
        }
        if world.local_destroy_block_is_immediate(hit.pos) {
            world.predict_local_destroy_block(hit.pos, sequence);
        } else {
            world.set_local_destroying_block_hit(hit.pos, hit.face);
        }
    }
    queue_player_action_command(
        counters,
        net_commands,
        PlayerActionKind::StartDestroyBlock,
        hit.pos,
        hit.face,
        sequence,
    );
    true
}

fn local_player_block_action_is_restricted(world: &WorldStore, pos: WorldBlockPos) -> bool {
    world.local_player_is_adventure()
        && !world.local_selected_main_hand_can_break_block_in_adventure_mode(pos)
}

fn stop_destroy_block(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    finished: LocalDestroyBlockFinished,
) {
    queue_player_action_command(
        counters,
        net_commands,
        PlayerActionKind::StopDestroyBlock,
        finished.pos,
        finished.face,
        finished.sequence,
    );
}

fn abort_destroy_block(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    direction: ProtocolDirection,
) {
    if let Some(pos) = world.take_local_destroying_block() {
        queue_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::AbortDestroyBlock,
            pos,
            direction,
            0,
        );
    }
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

fn local_player_hands_are_busy(input: &ClientInputState, world: &WorldStore) -> bool {
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
        || input.sign_editor_is_active_or_pending(world)
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
        || world.game_rules_screen().is_some()
    {
        return;
    }
    let Some(wheel) = wheel_steps_from_scroll(input, delta) else {
        return;
    };
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
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };

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
mod tests {
    use super::*;
    use bbb_net::{AttackEntityCommand, InteractEntityCommand, UseItemCommand, UseItemOnCommand};
    use bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID;
    use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID;
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    };
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
    use bbb_protocol::equipment_slots::{
        VANILLA_EQUIPMENT_SLOT_CHEST_ID, VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
    };
    use bbb_protocol::item_types::{
        VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_BONE_MEAL_ID,
        VANILLA_ITEM_BOW_ID, VANILLA_ITEM_BRUSH_ID, VANILLA_ITEM_BUNDLE_ID,
        VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID, VANILLA_ITEM_COMPASS_ID, VANILLA_ITEM_CROSSBOW_ID,
        VANILLA_ITEM_ENDER_PEARL_ID, VANILLA_ITEM_FIREWORK_ROCKET_ID, VANILLA_ITEM_FIRE_CHARGE_ID,
        VANILLA_ITEM_FISHING_ROD_ID, VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
        VANILLA_ITEM_GOAT_HORN_ID, VANILLA_ITEM_KNOWLEDGE_BOOK_ID, VANILLA_ITEM_MAP_ID,
        VANILLA_ITEM_POTION_ID, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_SHIELD_ID,
        VANILLA_ITEM_SPYGLASS_ID, VANILLA_ITEM_STONE_ID,
    };
    use bbb_protocol::packets::{
        AddEntity, AdventureModePredicateDataSummary, AdventureModePredicateSummary, AttackEntity,
        AttackRangeSummary, BlockEntityData, BlockHitResult as ProtocolBlockHitResult,
        BlockHolderSetSummary, BlockPos as ProtocolBlockPos, BlockPredicateSummary, BlockUpdate,
        CommonPlayerSpawnInfo, ConsumableSummary, DataComponentPatchSummary, EntityDataValue,
        EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EquippableSummary,
        GameEvent as ProtocolGameEvent, InitializeBorder, InstrumentSummary, InteractEntity,
        ItemStackSummary as ProtocolItemStackSummary, ItemStackTemplateSummary,
        LodestoneTargetSummary, OpenSignEditor, PickItemFromBlock, PickItemFromEntity, PlayLogin,
        PlayerAbilities, PlayerAction, PlayerHealth, RegistryTags, RemoveEntities, SetEntityData,
        SetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory, SoundEventSummary,
        TagNetworkPayload, ToolSummary, UpdateTags, UseItem, UseItemOn, Vec3d as ProtocolVec3d,
    };
    use bbb_world::{
        vanilla_entity_metadata_entity_accessors, BlockEntityRecord, BlockPos, ChunkColumn,
        ChunkPos, ChunkSection, ChunkState, ItemFoodProperties, LightData, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, SignBlockEntityTextState,
        WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldDimension,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use uuid::Uuid;

    const CROSSHAIR_BLOCK_POS: BlockPos = BlockPos { x: 0, y: 1, z: 3 };
    const VANILLA_GRASS_BLOCK_STATE_ID: i32 = 9;
    const VANILLA_WORLD_BORDER_ABSOLUTE_MAX_SIZE: i32 = 29_999_984;

    #[derive(Default)]
    struct RecordingAudioSink {
        entity_sounds: Vec<(bbb_world::SoundEntityEventState, Option<[f64; 3]>)>,
        positioned_sounds: Vec<bbb_world::SoundEventState>,
    }

    impl AudioEventSink for RecordingAudioSink {
        fn counters(&self) -> bbb_control::AudioCounters {
            bbb_control::AudioCounters::default()
        }

        fn set_sound_event_registry(&mut self, _registry: bbb_pack::SoundEventRegistry) {}

        fn set_jukebox_song_registry(&mut self, _registry: bbb_pack::JukeboxSongRegistry) {}

        fn play_local_sound(&mut self, _state: &bbb_world::LocalSoundEventState) {}

        fn play_positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
            self.positioned_sounds.push(state.clone());
        }

        fn play_entity_sound(
            &mut self,
            state: &bbb_world::SoundEntityEventState,
            position: Option<[f64; 3]>,
        ) {
            self.entity_sounds.push((state.clone(), position));
        }

        fn play_jukebox_song(&mut self, _state: &bbb_world::JukeboxLevelEventState) {}

        fn stop_jukebox_song(&mut self, _state: &bbb_world::JukeboxLevelEventState) {}

        fn stop_sound(&mut self, _state: &bbb_world::StopSoundEventState) {}

        fn set_music_gain(&mut self, _gain: f32) {}

        fn sync_ambient_loops(&mut self, _loops: &[bbb_world::AmbientLoopSoundState]) {}

        fn tick_entity_sound_positions(
            &mut self,
            _command: bbb_audio::TickEntitySoundPositionsCommand,
            _tickable_sounds: &[bbb_world::TickableEntitySoundState],
        ) {
        }
    }

    fn set_local_spectator(world: &mut WorldStore) {
        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });
        assert!(world.local_player_is_spectator());
    }

    fn set_local_adventure(world: &mut WorldStore) {
        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 2.0,
        });
        assert!(world.local_player_is_adventure());
    }

    fn set_flying_abilities(world: &mut WorldStore, flying_speed: f32) {
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed,
            walking_speed: 0.1,
        });
    }

    fn set_world_border_excluding_crosshair_block(world: &mut WorldStore) {
        world.apply_initialize_border(InitializeBorder {
            new_center_x: 0.0,
            new_center_z: 0.0,
            old_size: 2.0,
            new_size: 2.0,
            lerp_time: 0,
            new_absolute_max_size: VANILLA_WORLD_BORDER_ABSOLUTE_MAX_SIZE,
            warning_blocks: 5,
            warning_time: 15,
        });
        assert!(!world.world_border().contains_block_pos(CROSSHAIR_BLOCK_POS));
    }

    #[test]
    fn left_mouse_press_queues_main_hand_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(world.local_player().interaction.destroying_block, None);

        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.swing_commands_queued, 1);
    }

    #[test]
    fn unfocused_mouse_press_does_not_queue_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(false);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn sign_editor_mouse_press_does_not_queue_gameplay_input() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_empty_sign(BlockPos { x: 1, y: 2, z: 3 });
        world.apply_open_sign_editor(OpenSignEditor {
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            is_front_text: true,
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn left_mouse_release_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = WorldStore::new();
        world.set_local_destroying_block(BlockPos { x: 2, y: 65, z: -3 });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 2, y: 65, z: -3 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn right_mouse_press_without_block_queues_use_item() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        assert!(world.set_local_selected_hotbar_slot(4));
        install_consumable(&mut world, 4, 42, 1.6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 45.0,
                    x_rot: -20.0,
                },
                selected_hotbar_slot: 4,
            })
        );

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
        );

        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn right_mouse_instrument_use_starts_main_hand_cooldown_and_entity_sound() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 1.25,
                y: 64.0,
                z: -3.5,
            },
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        let mut horn = item_stack(VANILLA_ITEM_GOAT_HORN_ID, 1);
        horn.component_patch.added = 1;
        horn.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID];
        horn.component_patch.instrument_direct = Some(Box::new(InstrumentSummary {
            sound_event: bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:test_horn".to_string()),
                fixed_range_bits: Some(24.0_f32.to_bits()),
            },
            use_duration_bits: 3.25_f32.to_bits(),
            range_bits: 96.0_f32.to_bits(),
            description: "Test horn".to_string(),
            description_styled: None,
            description_nbt: None,
        }));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: horn,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_STONE_ID, 1),
        });
        let mut expected_seed_world = WorldStore::new();
        let expected_seed = expected_seed_world.next_sound_seed();
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            Some(InteractionHand::MainHand)
        );
        assert_eq!(
            world
                .cooldown("minecraft:goat_horn")
                .map(|cooldown| cooldown.duration),
            Some(65)
        );
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 45.0,
                    x_rot: -20.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert!(rx.try_recv().is_err());
        assert!(audio.positioned_sounds.is_empty());
        assert_eq!(audio.entity_sounds.len(), 1);
        let (sound, position) = &audio.entity_sounds[0];
        assert_eq!(sound.sound.kind, "direct");
        assert_eq!(sound.sound.location.as_deref(), Some("example:test_horn"));
        assert_eq!(sound.sound.fixed_range, Some(24.0));
        assert_eq!(sound.source, "record");
        assert_eq!(sound.entity_id, 42);
        assert_eq!(sound.volume, 6.0);
        assert_eq!(sound.pitch, 1.0);
        assert_eq!(sound.seed, expected_seed);
        assert_eq!(*position, Some([1.25, 64.0, -3.5]));
        assert_eq!(world.last_sound_entity(), Some(sound));
    }

    #[test]
    fn right_mouse_equippable_swap_mutates_inventory_and_stops_before_offhand() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        let sound = || SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: None,
        };
        let mut held = item_stack(VANILLA_ITEM_STONE_ID, 2);
        held.component_patch.added = 1;
        held.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID];
        held.component_patch.equippable = Some(EquippableSummary {
            slot_id: VANILLA_EQUIPMENT_SLOT_CHEST_ID,
            equip_sound: sound(),
            asset_id: None,
            camera_overlay: None,
            allowed_entities: None,
            dispensable: true,
            swappable: true,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: false,
            shearing_sound: sound(),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: held,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            item: item_stack(VANILLA_ITEM_SHIELD_ID, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_SHIELD_ID, 1),
        });
        world.set_default_item_max_stack_sizes(BTreeMap::from([(VANILLA_ITEM_SHIELD_ID, 1)]));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 45.0,
                    x_rot: -20.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(
            world
                .inventory()
                .player_slots
                .iter()
                .find(|slot| slot.slot == 0)
                .unwrap()
                .item
                .count,
            1
        );
        assert_eq!(
            world
                .inventory()
                .player_slots
                .iter()
                .find(|slot| slot.slot == 1)
                .unwrap()
                .item
                .item_id,
            Some(VANILLA_ITEM_SHIELD_ID)
        );
        assert_eq!(
            world
                .inventory()
                .player_slots
                .iter()
                .find(|slot| slot.slot == VANILLA_PLAYER_INVENTORY_CHEST_SLOT)
                .unwrap()
                .item
                .item_id,
            Some(VANILLA_ITEM_STONE_ID)
        );
        assert_eq!(
            world
                .inventory()
                .player_slots
                .iter()
                .find(|slot| slot.slot == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT)
                .unwrap()
                .item
                .item_id,
            Some(VANILLA_ITEM_SHIELD_ID)
        );
    }

    #[test]
    fn right_mouse_projectile_throw_consumes_cools_down_and_suppresses_local_sound() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        let mut pearl = item_stack(VANILLA_ITEM_ENDER_PEARL_ID, 2);
        pearl.component_patch.added = 1;
        pearl.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID];
        pearl.component_patch.use_cooldown_seconds_bits = Some(0.35_f32.to_bits());
        pearl.component_patch.use_cooldown_ticks = Some(7);
        pearl.component_patch.use_cooldown_group = Some("example:shared_throw".to_string());
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: pearl,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_STONE_ID, 1),
        });
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let _discarded_pitch = 0.4 / (expected_random.next_float() * 0.4 + 0.8);
        let mut expected_seed_world = WorldStore::new();
        let _discarded_seed = expected_seed_world.next_sound_seed();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 45.0,
                    x_rot: -20.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::OffHand)
                .unwrap()
                .item_id,
            Some(VANILLA_ITEM_STONE_ID)
        );
        assert_eq!(
            world
                .cooldown("example:shared_throw")
                .map(|cooldown| cooldown.duration),
            Some(7)
        );
        assert!(audio.positioned_sounds.is_empty());
        assert!(audio.entity_sounds.is_empty());
        assert!(world.client_audio().last_sound.is_none());
        assert_eq!(random.next_float(), expected_random.next_float());
        assert_eq!(world.next_sound_seed(), expected_next_seed);
    }

    #[test]
    fn right_mouse_bow_use_and_release_follow_ammo_threshold_and_suppressed_sound() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 20.0,
            x_rot: -10.0,
            ..LocalPlayerPoseState::default()
        });
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:arrows".to_string(),
                    entries: vec![VANILLA_ITEM_ARROW_ID],
                }],
            }],
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_BOW_ID, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: item_stack(VANILLA_ITEM_ARROW_ID, 4),
        });
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let _discarded_pitch = 1.0 / (expected_random.next_float() * 0.4 + 1.2) + 0.1075 * 0.5;
        let mut expected_seed_world = WorldStore::new();
        let _discarded_seed = expected_seed_world.next_sound_seed();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut counters = NetCounters::default();
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            Some(InteractionHand::MainHand)
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 20.0,
                    x_rot: -10.0,
                },
                selected_hotbar_slot: 0,
            })
        );

        world.advance_local_using_item_ticks(3);
        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert!(!world.local_player().interaction.using_item);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            world
                .inventory()
                .player_slots
                .iter()
                .find(|slot| slot.slot == 9)
                .map(|slot| slot.item.count),
            Some(4)
        );
        assert!(audio.positioned_sounds.is_empty());
        assert!(audio.entity_sounds.is_empty());
        assert_eq!(random.next_float(), expected_random.next_float());
        assert_eq!(world.next_sound_seed(), expected_next_seed);
    }

    #[test]
    fn right_mouse_crossbow_draw_uses_held_firework_without_local_loading_side_effects() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 20.0,
            x_rot: -10.0,
            ..LocalPlayerPoseState::default()
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_CROSSBOW_ID, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_FIREWORK_ROCKET_ID, 1),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_seed_world = WorldStore::new();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            Some(InteractionHand::MainHand)
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 20.0,
                    x_rot: -10.0,
                },
                selected_hotbar_slot: 0,
            })
        );

        advance_local_using_item_ticks_with_effects(
            &mut world,
            &mut counters,
            &commands,
            30,
            &mut random,
            &mut audio_events,
        );
        assert_eq!(world.local_player().interaction.using_item_ticks, 30);
        assert!(world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .component_patch
            .charged_projectiles_items
            .is_empty());

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert!(!world.local_player().interaction.using_item);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                ..
            })
        ));
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(audio.positioned_sounds.is_empty());
        assert!(audio.entity_sounds.is_empty());
        assert_eq!(random.next_float(), expected_random.next_float());
        assert_eq!(world.next_sound_seed(), expected_next_seed);
    }

    #[test]
    fn charged_crossbow_use_stays_server_owned_without_starting_or_swinging() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let mut crossbow = item_stack(VANILLA_ITEM_CROSSBOW_ID, 1);
        crossbow.component_patch.added = 1;
        crossbow
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID);
        crossbow
            .component_patch
            .charged_projectiles_items
            .push(ItemStackTemplateSummary {
                item_id: VANILLA_ITEM_ARROW_ID,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: crossbow.clone(),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_seed_world = WorldStore::new();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));
        assert!(rx.try_recv().is_err());
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&crossbow)
        );
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(audio.positioned_sounds.is_empty());
        assert!(audio.entity_sounds.is_empty());
        assert_eq!(random.next_float(), expected_random.next_float());
        assert_eq!(world.next_sound_seed(), expected_next_seed);
    }

    #[test]
    fn fishing_rod_cast_and_retrieve_swing_without_local_bobber_or_sound() {
        for retrieving in [false, true] {
            let (tx, mut rx) = mpsc::channel(2);
            let commands = Some(tx);
            let mut input = ClientInputState::new(true);
            let mut world = WorldStore::new();
            world.apply_login(&PlayLogin {
                player_id: 42,
                hardcore: false,
                levels: vec!["minecraft:overworld".to_string()],
                max_players: 20,
                chunk_radius: 8,
                simulation_distance: 6,
                reduced_debug_info: false,
                show_death_screen: true,
                do_limited_crafting: false,
                common_spawn_info: CommonPlayerSpawnInfo {
                    dimension_type_id: 0,
                    dimension: "minecraft:overworld".to_string(),
                    seed: 0,
                    game_type: 0,
                    previous_game_type: -1,
                    is_debug: false,
                    is_flat: false,
                    last_death_location: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                },
                enforces_secure_chat: true,
            });
            world.set_local_player_pose(LocalPlayerPoseState {
                y_rot: 20.0,
                x_rot: -10.0,
                ..LocalPlayerPoseState::default()
            });
            let rod = item_stack(VANILLA_ITEM_FISHING_ROD_ID, 1);
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: 0,
                item: rod.clone(),
            });
            if retrieving {
                world.apply_add_entity(AddEntity {
                    id: 88,
                    uuid: Uuid::from_u128(88),
                    entity_type_id: VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
                    position: ProtocolVec3d::default(),
                    delta_movement: ProtocolVec3d::default(),
                    x_rot: 0.0,
                    y_rot: 0.0,
                    y_head_rot: 0.0,
                    data: 42,
                });
            }
            assert_eq!(
                world.local_player_fishing_bobber_id(),
                retrieving.then_some(88)
            );
            let mut counters = NetCounters::default();
            let mut random = LevelEventSoundRandomState::with_seed(0);
            let mut expected_random = LevelEventSoundRandomState::with_seed(0);
            let _discarded_pitch = 0.4 / (expected_random.next_float() * 0.4 + 0.8);
            let expected_next_random = expected_random.next_float();
            let mut expected_seed_world = WorldStore::new();
            let _discarded_seed = expected_seed_world.next_sound_seed();
            let expected_next_seed = expected_seed_world.next_sound_seed();
            let mut audio = RecordingAudioSink::default();
            let mut audio_events =
                Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

            handle_mouse_input_at_partial_tick_with_effects(
                &mut input,
                &mut world,
                &mut counters,
                &commands,
                MouseButton::Right,
                ElementState::Pressed,
                1.0,
                &mut random,
                &mut audio_events,
            );
            drop(audio_events);

            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::UseItem(UseItemCommand {
                    packet: UseItem {
                        hand: InteractionHand::MainHand,
                        sequence: 1,
                        y_rot: 20.0,
                        x_rot: -10.0,
                    },
                    selected_hotbar_slot: 0,
                })
            );
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::MainHand)
            );
            assert!(rx.try_recv().is_err());
            assert!(!world.local_player().interaction.using_item);
            assert_eq!(
                world.local_item_in_hand(InteractionHand::MainHand),
                Some(&rod)
            );
            assert_eq!(
                world.local_player_fishing_bobber_id(),
                retrieving.then_some(88)
            );
            assert_eq!(counters.use_item_commands_queued, 1);
            assert_eq!(counters.swing_commands_queued, 1);
            assert!(audio.positioned_sounds.is_empty());
            assert!(audio.entity_sounds.is_empty());
            assert!(world.client_audio().last_sound.is_none());
            assert_eq!(random.next_float(), expected_next_random);
            assert_eq!(world.next_sound_seed(), expected_next_seed);
        }
    }

    #[test]
    fn firework_rocket_passes_on_ground_and_swings_without_local_launch_while_flying() {
        for fall_flying in [false, true] {
            let (tx, mut rx) = mpsc::channel(2);
            let commands = Some(tx);
            let mut input = ClientInputState::new(true);
            let mut world = WorldStore::new();
            world.apply_login(&PlayLogin {
                player_id: 42,
                hardcore: false,
                levels: vec!["minecraft:overworld".to_string()],
                max_players: 20,
                chunk_radius: 8,
                simulation_distance: 6,
                reduced_debug_info: false,
                show_death_screen: true,
                do_limited_crafting: false,
                common_spawn_info: CommonPlayerSpawnInfo {
                    dimension_type_id: 0,
                    dimension: "minecraft:overworld".to_string(),
                    seed: 0,
                    game_type: 0,
                    previous_game_type: -1,
                    is_debug: false,
                    is_flat: false,
                    last_death_location: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                },
                enforces_secure_chat: true,
            });
            world.apply_add_entity(AddEntity {
                id: 42,
                uuid: Uuid::from_u128(42),
                entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
                position: ProtocolVec3d::default(),
                delta_movement: ProtocolVec3d::default(),
                x_rot: 0.0,
                y_rot: 0.0,
                y_head_rot: 0.0,
                data: 0,
            });
            if fall_flying {
                let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
                assert!(world.apply_set_entity_data(SetEntityData {
                    id: 42,
                    values: vec![EntityDataValue {
                        data_id: accessors.shared_flags_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                        value: EntityDataValueKind::Byte(
                            accessors.shared_flags_default | accessors.fall_flying_mask,
                        ),
                    }],
                }));
            }
            world.set_local_player_pose(LocalPlayerPoseState::default());
            let rocket = item_stack(VANILLA_ITEM_FIREWORK_ROCKET_ID, 2);
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: 0,
                item: rocket.clone(),
            });
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
                item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
            });
            let entity_count = world.entity_count();
            let mut counters = NetCounters::default();
            let mut random = LevelEventSoundRandomState::with_seed(0);
            let mut audio = RecordingAudioSink::default();
            let mut audio_events =
                Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

            handle_mouse_input_at_partial_tick_with_effects(
                &mut input,
                &mut world,
                &mut counters,
                &commands,
                MouseButton::Right,
                ElementState::Pressed,
                1.0,
                &mut random,
                &mut audio_events,
            );
            drop(audio_events);

            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::UseItem(UseItemCommand {
                    packet: UseItem {
                        hand: InteractionHand::MainHand,
                        sequence: 1,
                        y_rot: 0.0,
                        x_rot: 0.0,
                    },
                    selected_hotbar_slot: 0,
                })
            );
            if fall_flying {
                assert_eq!(
                    rx.try_recv().unwrap(),
                    NetCommand::Swing(InteractionHand::MainHand)
                );
            } else {
                assert_eq!(
                    rx.try_recv().unwrap(),
                    NetCommand::UseItem(UseItemCommand {
                        packet: UseItem {
                            hand: InteractionHand::OffHand,
                            sequence: 2,
                            y_rot: 0.0,
                            x_rot: 0.0,
                        },
                        selected_hotbar_slot: 0,
                    })
                );
            }
            assert!(rx.try_recv().is_err());
            assert_eq!(
                counters.use_item_commands_queued,
                if fall_flying { 1 } else { 2 }
            );
            assert_eq!(counters.swing_commands_queued, usize::from(fall_flying));
            assert_eq!(
                world.local_player().interaction.using_item_hand,
                (!fall_flying).then_some(InteractionHand::OffHand)
            );
            assert_eq!(
                world.local_item_in_hand(InteractionHand::MainHand),
                Some(&rocket)
            );
            assert_eq!(world.entity_count(), entity_count);
            assert_eq!(audio.positioned_sounds.len(), usize::from(!fall_flying));
            assert!(audio.entity_sounds.is_empty());
        }
    }

    #[test]
    fn empty_map_swings_without_local_conversion_sound_or_offhand_fallback() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let map = item_stack(VANILLA_ITEM_MAP_ID, 2);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: map.clone(),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    y_rot: 0.0,
                    x_rot: 0.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&map)
        );
        assert!(world.map_items().is_empty());
        assert!(audio.positioned_sounds.is_empty());
        assert!(audio.entity_sounds.is_empty());
    }

    #[test]
    fn offhand_bundle_use_tick_drops_content_with_main_hand_swing_and_ordered_sound_random() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        let position = ProtocolVec3d {
            x: 1.5,
            y: 64.0,
            z: -2.25,
        };
        world.set_local_player_pose(LocalPlayerPoseState {
            position,
            y_rot: 20.0,
            x_rot: -10.0,
            ..LocalPlayerPoseState::default()
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: ProtocolItemStackSummary {
                item_id: Some(VANILLA_ITEM_BUNDLE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
                    bundle_contents_items: vec![ItemStackTemplateSummary {
                        item_id: VANILLA_ITEM_STONE_ID,
                        count: 3,
                        component_patch: DataComponentPatchSummary::default(),
                    }],
                    bundle_contents_item_count: Some(1),
                    ..DataComponentPatchSummary::default()
                },
            },
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_pitch = 0.8 + expected_random.next_float() * 0.4;
        let _discarded_drop_pitch = 0.8 + expected_random.next_float() * 0.4;
        let expected_next_random = expected_random.next_float();
        let mut expected_seed_world = WorldStore::new();
        let expected_sound_seed = expected_seed_world.next_sound_seed();
        let _discarded_drop_seed = expected_seed_world.next_sound_seed();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            Some(InteractionHand::OffHand)
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::OffHand,
                    sequence: 1,
                    y_rot: 20.0,
                    x_rot: -10.0,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::OffHand)
        );

        advance_local_using_item_ticks_with_effects(
            &mut world,
            &mut counters,
            &commands,
            1,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 2);
        assert_eq!(world.local_player().interaction.using_item_ticks, 1);
        let bundle = world
            .local_item_in_hand(InteractionHand::OffHand)
            .expect("offhand bundle");
        assert_eq!(bundle.component_patch.bundle_contents_item_count, None);
        assert!(bundle.component_patch.bundle_contents_items.is_empty());
        assert!(!bundle
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID));
        assert_eq!(audio.positioned_sounds.len(), 1);
        assert_eq!(
            audio.positioned_sounds[0].sound.location.as_deref(),
            Some("minecraft:item.bundle.remove_one")
        );
        assert_eq!(audio.positioned_sounds[0].source, "player");
        assert_eq!(audio.positioned_sounds[0].position, position);
        assert_eq!(audio.positioned_sounds[0].volume, 0.8);
        assert_eq!(audio.positioned_sounds[0].pitch, expected_pitch);
        assert_eq!(audio.positioned_sounds[0].seed, expected_sound_seed);
        assert!(audio.entity_sounds.is_empty());
        assert_eq!(random.next_float(), expected_next_random);
        assert_eq!(world.next_sound_seed(), expected_next_seed);
    }

    #[test]
    fn projectile_weapon_without_arrow_falls_through_to_offhand_spyglass_use() {
        for weapon_item_id in [VANILLA_ITEM_BOW_ID, VANILLA_ITEM_CROSSBOW_ID] {
            let (tx, mut rx) = mpsc::channel(2);
            let commands = Some(tx);
            let mut input = ClientInputState::new(true);
            let mut world = WorldStore::new();
            world.set_local_player_pose(LocalPlayerPoseState::default());
            world.apply_update_tags(UpdateTags {
                registries: vec![RegistryTags {
                    registry: "minecraft:item".to_string(),
                    tags: vec![TagNetworkPayload {
                        tag: "minecraft:arrows".to_string(),
                        entries: vec![VANILLA_ITEM_ARROW_ID],
                    }],
                }],
            });
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: 0,
                item: item_stack(weapon_item_id, 1),
            });
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
                item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
            });
            let mut counters = NetCounters::default();
            let mut random = LevelEventSoundRandomState::with_seed(0);
            let mut audio = RecordingAudioSink::default();
            let mut audio_events =
                Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

            handle_mouse_input_at_partial_tick_with_effects(
                &mut input,
                &mut world,
                &mut counters,
                &commands,
                MouseButton::Right,
                ElementState::Pressed,
                1.0,
                &mut random,
                &mut audio_events,
            );
            drop(audio_events);

            for (sequence, hand) in [
                (1, InteractionHand::MainHand),
                (2, InteractionHand::OffHand),
            ] {
                assert_eq!(
                    rx.try_recv().unwrap(),
                    NetCommand::UseItem(UseItemCommand {
                        packet: UseItem {
                            hand,
                            sequence,
                            y_rot: 0.0,
                            x_rot: 0.0,
                        },
                        selected_hotbar_slot: 0,
                    })
                );
            }
            assert!(rx.try_recv().is_err());
            assert_eq!(counters.use_item_commands_queued, 2);
            assert_eq!(counters.swing_commands_queued, 0);
            assert_eq!(
                world.local_player().interaction.using_item_hand,
                Some(InteractionHand::OffHand)
            );
            assert_eq!(
                audio.positioned_sounds[0].sound.location.as_deref(),
                Some("minecraft:item.spyglass.use")
            );
        }
    }

    #[test]
    fn knowledge_book_consumes_before_result_and_preserves_hand_fallback() {
        for (recipe_keys, expected_use_item_commands, offhand_started) in [
            (Vec::new(), 2, true),
            (vec!["minecraft:bread".to_string()], 1, false),
        ] {
            let (tx, mut rx) = mpsc::channel(3);
            let commands = Some(tx);
            let mut input = ClientInputState::new(true);
            let mut world = WorldStore::new();
            world.set_local_player_pose(LocalPlayerPoseState::default());
            let mut book = item_stack(VANILLA_ITEM_KNOWLEDGE_BOOK_ID, 1);
            book.component_patch.added = 1;
            book.component_patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID);
            book.component_patch.recipe_keys = Some(recipe_keys);
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: 0,
                item: book,
            });
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
                item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
            });
            let mut counters = NetCounters::default();
            let mut random = LevelEventSoundRandomState::with_seed(0);
            let mut audio = RecordingAudioSink::default();
            let mut audio_events =
                Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

            handle_mouse_input_at_partial_tick_with_effects(
                &mut input,
                &mut world,
                &mut counters,
                &commands,
                MouseButton::Right,
                ElementState::Pressed,
                1.0,
                &mut random,
                &mut audio_events,
            );
            drop(audio_events);

            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::UseItem(UseItemCommand {
                    packet: UseItem {
                        hand: InteractionHand::MainHand,
                        sequence: 1,
                        y_rot: 0.0,
                        x_rot: 0.0,
                    },
                    selected_hotbar_slot: 0,
                })
            );
            if offhand_started {
                assert_eq!(
                    rx.try_recv().unwrap(),
                    NetCommand::UseItem(UseItemCommand {
                        packet: UseItem {
                            hand: InteractionHand::OffHand,
                            sequence: 2,
                            y_rot: 0.0,
                            x_rot: 0.0,
                        },
                        selected_hotbar_slot: 0,
                    })
                );
            } else {
                assert_eq!(
                    rx.try_recv().unwrap(),
                    NetCommand::Swing(InteractionHand::MainHand)
                );
            }
            assert!(rx.try_recv().is_err());
            assert_eq!(
                counters.use_item_commands_queued,
                expected_use_item_commands
            );
            assert_eq!(
                counters.swing_commands_queued,
                usize::from(!offhand_started)
            );
            assert!(world
                .local_item_in_hand(InteractionHand::MainHand)
                .is_none());
            assert_eq!(
                world.local_player().interaction.using_item_hand,
                offhand_started.then_some(InteractionHand::OffHand)
            );
            assert_eq!(audio.positioned_sounds.len(), usize::from(offhand_started));
        }
    }

    #[test]
    fn right_mouse_spyglass_use_and_release_emit_ordered_local_sounds() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 1.5,
                y: 64.0,
                z: -2.25,
            },
            ..LocalPlayerPoseState::default()
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_SPYGLASS_ID, 1),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        for state in [ElementState::Pressed, ElementState::Released] {
            handle_mouse_input_at_partial_tick_with_effects(
                &mut input,
                &mut world,
                &mut counters,
                &commands,
                MouseButton::Right,
                state,
                1.0,
                &mut random,
                &mut audio_events,
            );
        }
        drop(audio_events);

        assert!(!world.local_player().interaction.using_item);
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::ReleaseUseItem,
                ..
            })
        ));
        assert!(rx.try_recv().is_err());
        assert_eq!(
            audio
                .positioned_sounds
                .iter()
                .map(|sound| sound.sound.location.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("minecraft:item.spyglass.use"),
                Some("minecraft:item.spyglass.stop_using"),
            ]
        );
        assert!(audio.positioned_sounds.iter().all(|sound| {
            sound.source == "player"
                && sound.position
                    == ProtocolVec3d {
                        x: 1.5,
                        y: 64.0,
                        z: -2.25,
                    }
                && sound.volume == 1.0
                && sound.pitch == 1.0
        }));
        assert_ne!(
            audio.positioned_sounds[0].seed,
            audio.positioned_sounds[1].seed
        );
        assert!(audio.entity_sounds.is_empty());
    }

    #[test]
    fn right_mouse_press_without_target_uses_offhand_when_selected_hotbar_slot_is_empty() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        install_consumable(&mut world, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, 99, 1.6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::OffHand,
                    sequence: 1,
                    y_rot: 45.0,
                    x_rot: -20.0,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn right_mouse_press_without_target_or_items_only_starts_repeat_delay() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_without_target_honors_offhand_enabled_feature_gate() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        install_consumable(&mut world, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, 99, 1.6);
        world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec!["minecraft:trade_rebalance".to_string()],
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn full_food_consumable_blocks_main_hand_and_falls_through_to_offhand() {
        fn full_food_world() -> WorldStore {
            let mut world = WorldStore::new();
            world.set_local_player_pose(LocalPlayerPoseState {
                y_rot: 45.0,
                x_rot: -20.0,
                ..LocalPlayerPoseState::default()
            });
            world.set_default_item_component_type_ids(BTreeMap::from([
                (
                    42,
                    BTreeSet::from([
                        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
                        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                    ]),
                ),
                (
                    99,
                    BTreeSet::from([VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID]),
                ),
            ]));
            world.set_default_item_consumables(BTreeMap::from([
                (
                    42,
                    ConsumableSummary {
                        consume_seconds: 1.6,
                        ..ConsumableSummary::default()
                    },
                ),
                (
                    99,
                    ConsumableSummary {
                        consume_seconds: 1.6,
                        ..ConsumableSummary::default()
                    },
                ),
            ]));
            world.set_default_item_food_properties(BTreeMap::from([(
                42,
                ItemFoodProperties {
                    nutrition: 4,
                    saturation_bits: 2.4_f32.to_bits(),
                    can_always_eat: false,
                },
            )]));
            world.apply_player_health(PlayerHealth {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            });
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: 0,
                item: item_stack(42, 1),
            });
            world
        }

        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = full_food_world();
        let mut counters = NetCounters::default();
        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    ..
                },
                selected_hotbar_slot: 0,
            })
        ));

        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = full_food_world();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();
        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );
        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 2);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 1,
                    ..
                },
                selected_hotbar_slot: 0,
            })
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::OffHand,
                    sequence: 2,
                    ..
                },
                selected_hotbar_slot: 0,
            })
        ));
    }

    #[test]
    fn spectator_right_mouse_press_without_target_does_not_use_item() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 0);

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            4,
        );

        assert_eq!(counters.use_item_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn left_mouse_press_on_entity_queues_attack_and_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.attack_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(AttackEntityCommand {
                packet: AttackEntity { entity_id: 123 },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn left_mouse_press_on_extended_attack_range_entity_queues_attack_and_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity_at(123, [0.0, 1.0, 4.0]);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack_with_attack_range(42, 1, 0.0, 4.5),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.attack_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(AttackEntityCommand {
                packet: AttackEntity { entity_id: 123 },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn left_mouse_press_on_entity_outside_selected_attack_range_swings_without_attack() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack_with_attack_range(42, 1, 0.0, 1.0),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert!(input.destroy_block_held);
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn left_mouse_press_with_piercing_weapon_queues_stab_and_swing_on_entity() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_selected_piercing_weapon(&mut world, 42);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::Stab,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn spectator_left_mouse_press_on_entity_queues_spectate_only() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.spectate_entity_commands_queued, 1);
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SpectateEntity(SpectateEntity { entity_id: 123 })
        );
    }

    #[test]
    fn spectator_left_mouse_press_with_piercing_weapon_queues_spectate_only() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_selected_piercing_weapon(&mut world, 42);
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(counters.spectate_entity_commands_queued, 1);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::SpectateEntity(SpectateEntity { entity_id: 123 })
        );
    }

    #[test]
    fn left_mouse_press_while_using_item_does_not_attack_or_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        world.set_local_using_item(true);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn local_attack_entity_carries_selected_slot_and_resets_both_strength_clocks() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = local_attack_world_with_player_and_target(123);
        assert!(world.set_local_selected_hotbar_slot(4));
        world.advance_client_level_entity_ticks_with_observation(6);
        assert_eq!(world.local_player_attack_strength_ticks(), 6);
        assert_eq!(world.local_player_item_swap_ticks(), 6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player_attack_strength_ticks(), 0);
        assert_eq!(world.local_player_item_swap_ticks(), 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(AttackEntityCommand {
                packet: AttackEntity { entity_id: 123 },
                selected_hotbar_slot: 4,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn local_attack_charge_and_feature_gates_emit_no_action_or_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = local_attack_world_with_player_and_target(123);
        world.set_default_item_minimum_attack_charges(BTreeMap::from([(42, 1.0)]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        world.advance_client_level_entity_ticks_with_observation(5);
        assert_eq!(world.local_player_attack_strength_ticks(), 4);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        world.advance_client_level_entity_ticks_with_observation(1);
        world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec!["minecraft:trade_rebalance".to_string()],
        });
        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec!["minecraft:vanilla".to_string()],
        });
        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(counters.attack_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(_)
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn local_attack_miss_time_uses_screen_sentinel_and_clears_on_release() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = local_attack_world_with_player_and_target(123);
        assert_eq!(
            world.apply_remove_entities(RemoveEntities {
                entity_ids: vec![123],
            }),
            1
        );
        world.advance_client_level_entity_ticks_with_observation(6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(input.attack_miss_time_ticks, 10);
        assert_eq!(world.local_player_attack_strength_ticks(), 0);
        assert_eq!(world.local_player_item_swap_ticks(), 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(rx.try_recv().is_err());

        input.open_debug_pause_screen_with_menu();
        crate::input::advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &None,
            std::time::Instant::now(),
            5,
        );
        assert_eq!(input.attack_miss_time_ticks, 10_000);
        input.close_debug_pause_screen();
        crate::input::advance_player_input_for_client_ticks(
            &mut input,
            &mut world,
            &mut counters,
            &None,
            std::time::Instant::now(),
            5,
        );
        assert_eq!(input.attack_miss_time_ticks, 9_995);

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &None,
            MouseButton::Left,
            ElementState::Released,
        );
        assert_eq!(input.attack_miss_time_ticks, 0);

        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 1.0,
        });
        assert!(world.local_player_is_creative());
        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );
        assert_eq!(input.attack_miss_time_ticks, 0);
        assert_eq!(counters.swing_commands_queued, 2);
    }

    #[test]
    fn local_attack_boat_directional_input_blocks_before_spectator_or_item_routing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = local_attack_world_with_player_and_target(123);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 50,
            passenger_ids: vec![42],
        }));
        assert_eq!(
            world.local_player_first_passenger_root_boat_vehicle_id(),
            Some(50)
        );
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(counters.attack_entity_commands_queued, 0);
        assert_eq!(counters.spectate_entity_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_boat_directional_input_blocks_item_use() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.forward = true;
        let mut world = local_attack_world_with_player_and_target(123);
        world.apply_remove_entities(RemoveEntities {
            entity_ids: vec![123],
        });
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 50,
            passenger_ids: vec![42],
        }));
        install_consumable(&mut world, 0, 42, 1.6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn local_attack_piercing_weapon_resets_only_attack_strength() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = local_attack_world_with_player_and_target(123);
        set_selected_piercing_weapon(&mut world, 42);
        world.advance_client_level_entity_ticks_with_observation(6);
        assert_eq!(world.local_player_attack_strength_ticks(), 5);
        assert_eq!(world.local_player_item_swap_ticks(), 5);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player_attack_strength_ticks(), 0);
        assert_eq!(world.local_player_item_swap_ticks(), 5);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::Stab,
                ..
            })
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn left_mouse_press_on_block_queues_start_destroy_and_swing() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert!(input.destroy_block_held);
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn adventure_mode_block_destroy_requires_matching_can_break_predicate() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        set_local_adventure(&mut world);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(1, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        input.destroy_block_held = false;
        set_selected_can_break(&mut world, "minecraft:grass_block");
        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 2);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                ..
            })
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn survival_instant_left_mouse_press_predicts_destroy_without_delay() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.set_default_block_destroy_profiles(BTreeMap::from([(
            "minecraft:grass_block".to_string(),
            WorldBlockDestroyProfile {
                destroy_time_tenths: Some(0),
                requires_correct_tool: false,
            },
        )]));
        assert!(world.local_destroy_block_is_immediate(CROSSHAIR_BLOCK_POS));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 0);
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            bbb_world::vanilla_air_block_state_id()
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn survival_start_runs_redstone_attack_prediction_before_start_command() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        let unlit = test_block_state_id("minecraft:redstone_ore", &[("lit", "false")]);
        let lit = test_block_state_id("minecraft:redstone_ore", &[("lit", "true")]);
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: unlit,
        }));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            lit
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 1);
        assert_eq!(
            world.local_block_predictions()[0].server_block_state_id,
            unlit
        );
        assert_eq!(
            world.local_block_predictions()[0].predicted_block_state_id,
            lit
        );
        assert_eq!(
            input.pending_local_player_action_particles,
            [
                crate::particle_runtime::LocalPlayerActionParticleState::BlockAttack(
                    bbb_world::LocalBlockAttackEffect::RedstoneOre {
                        pos: CROSSHAIR_BLOCK_POS,
                    },
                )
            ]
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            1,
        );

        assert_eq!(input.pending_local_player_action_particles.len(), 2);
        assert!(matches!(
            input.pending_local_player_action_particles[0],
            crate::particle_runtime::LocalPlayerActionParticleState::BlockAttack(_)
        ));
        assert!(matches!(
            input.pending_local_player_action_particles[1],
            crate::particle_runtime::LocalPlayerActionParticleState::DestroyHit(_)
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn left_mouse_press_with_piercing_weapon_queues_stab_and_swing_on_block() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        set_selected_piercing_weapon(&mut world, 42);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::Stab,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn left_mouse_press_on_block_outside_world_border_swings_without_destroying() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        set_world_border_excluding_crosshair_block(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spectator_left_mouse_press_on_block_does_not_start_destroy_or_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.spectate_entity_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn left_mouse_press_while_using_item_does_not_start_destroy() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.set_local_using_item(true);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_retargets_destroy_block_with_abort_and_start() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        let old_pos = BlockPos { x: 2, y: 65, z: -3 };
        world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            0,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(counters.player_action_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 2, y: 65, z: -3 },
                direction: ProtocolDirection::North,
                sequence: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
    }

    #[test]
    fn held_left_mouse_retarget_to_block_outside_world_border_keeps_current_destroy_state() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        set_world_border_excluding_crosshair_block(&mut world);
        let old_pos = BlockPos { x: 2, y: 65, z: -3 };
        world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            1,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(old_pos)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::South)
        );
        assert_eq!(counters.player_action_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_while_using_item_does_not_continue_or_abort_destroy() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        let old_pos = BlockPos { x: 2, y: 65, z: -3 };
        world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
        world.set_local_using_item(true);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            1,
        );

        assert!(input.destroy_block_held);
        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(old_pos)
        );
        assert_eq!(counters.player_action_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_with_piercing_weapon_does_not_continue_or_abort_destroy() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        let old_pos = BlockPos { x: 2, y: 65, z: -3 };
        world.set_local_destroying_block_hit(old_pos, ProtocolDirection::South);
        set_selected_piercing_weapon(&mut world, 42);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            1,
        );

        assert!(input.destroy_block_held);
        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(old_pos)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::South)
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Released,
        );

        assert!(!input.destroy_block_held);
        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(old_pos)
        );
        assert_eq!(counters.player_action_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_same_target_restarts_destroy_when_main_hand_item_changes() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(43, 1),
        });
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            1,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_face,
            Some(ProtocolDirection::North)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(counters.player_action_commands_queued, 2);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(input.pending_local_player_action_particles.len(), 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_miss_aborts_destroying_block() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = WorldStore::new();
        world.set_local_destroying_block_hit(
            BlockPos { x: 2, y: 65, z: -3 },
            ProtocolDirection::South,
        );
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            0,
        );

        assert!(input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(world.local_player().interaction.destroying_block_face, None);
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::AbortDestroyBlock,
                pos: ProtocolBlockPos { x: 2, y: 65, z: -3 },
                direction: ProtocolDirection::Down,
                sequence: 0,
            })
        );
    }

    #[test]
    fn held_left_mouse_same_target_finishes_destroy_with_stop() {
        let (tx, mut rx) = mpsc::channel(20);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            18,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            bbb_world::vanilla_air_block_state_id()
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 18);
        for _ in 0..17 {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::MainHand)
            );
        }
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StopDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(input.pending_local_player_action_particles.len(), 17);
    }

    #[test]
    fn held_left_mouse_same_target_advances_destroy_only_on_client_ticks() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            0,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_progress,
            0
        );
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_same_target_updates_local_destroy_stage_on_client_tick() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        let mut counters = NetCounters::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            None,
            1.0,
            1,
        );

        assert_eq!(
            world.local_player().interaction.destroying_block,
            Some(CROSSHAIR_BLOCK_POS)
        );
        assert!(world.local_player().interaction.destroying_block_progress > 0);
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            Some(0)
        );
        assert_eq!(world.local_player().interaction.destroying_block_ticks, 1);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(input.pending_local_player_action_particles.len(), 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_same_target_emits_vanilla_block_hit_sound_every_four_ticks() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.destroy_block_held = true;
        let mut world = world_with_crosshair_block();
        world.set_default_block_sound_profiles(BTreeMap::from([(
            "minecraft:grass_block".to_string(),
            WorldBlockSoundProfile {
                break_sound: "minecraft:block.grass.break".to_string(),
                hit_sound: "minecraft:block.grass.hit".to_string(),
                volume: 1.0,
                pitch: 1.2,
            },
        )]));
        world.set_local_destroying_block_hit(CROSSHAIR_BLOCK_POS, ProtocolDirection::North);
        let mut counters = NetCounters::default();
        let mut audio = RecordingAudioSink::default();

        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(&mut audio),
            1.0,
            1,
        );
        advance_destroying_block_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(&mut audio),
            1.0,
            1,
        );

        assert_eq!(audio.positioned_sounds.len(), 1);
        let sound = &audio.positioned_sounds[0];
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.grass.hit")
        );
        assert_eq!(sound.source, "block");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 0.5,
                y: 1.5,
                z: 3.5,
            }
        );
        assert_eq!(sound.volume, 0.25);
        assert_eq!(sound.pitch, 0.6);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 2);
        assert_eq!(input.pending_local_player_action_particles.len(), 2);
        for _ in 0..2 {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::MainHand)
            );
        }
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn creative_left_mouse_press_queues_start_without_destroy_state() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(input.destroy_block_held);
        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(
            world.local_player().interaction.destroying_block_stage,
            None
        );
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            bbb_world::vanilla_air_block_state_id()
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn creative_tool_flag_suppresses_local_destroy_but_still_queues_start() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let mut stack = item_stack(42, 1);
        stack.component_patch.added = 1;
        stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID];
        stack.component_patch.tool = Some(ToolSummary {
            rules: Vec::new(),
            default_mining_speed_bits: 1.0_f32.to_bits(),
            damage_per_block: 2,
            can_destroy_blocks_in_creative: false,
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: stack,
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert!(input.destroy_block_held);
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            VANILLA_GRASS_BLOCK_STATE_ID
        );
        assert!(world.local_block_predictions().is_empty());
        assert_eq!(world.local_player().interaction.destroy_delay_ticks, 5);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                ..
            })
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn left_mouse_press_uses_active_camera_entity_raycast() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            ..LocalPlayerPoseState::default()
        });
        world.apply_add_entity(AddEntity {
            id: 200,
            uuid: Uuid::from_u128(200),
            entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.apply_add_entity(AddEntity {
            id: 123,
            uuid: Uuid::from_u128(123),
            entity_type_id: VANILLA_ENTITY_TYPE_MINECART_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 200 }));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
        );

        assert_eq!(world.local_player().interaction.destroying_block, None);
        assert_eq!(counters.attack_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::AttackEntity(AttackEntityCommand {
                packet: AttackEntity { entity_id: 123 },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn right_mouse_press_on_entity_queues_interact_with_relative_hit_location() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sneak = true;
        let mut world = world_with_crosshair_entity(123);
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert_eq!(counters.interact_entity_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntityCommand {
                packet: InteractEntity {
                    entity_id: 123,
                    hand: InteractionHand::MainHand,
                    location: ProtocolVec3d {
                        x: 0.0,
                        y: 0.6200000047683716,
                        z: -0.49000000953674316,
                    },
                    using_secondary_action: true,
                },
                selected_hotbar_slot: 4,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntityCommand {
                packet: InteractEntity {
                    entity_id: 123,
                    hand: InteractionHand::OffHand,
                    location: ProtocolVec3d {
                        x: 0.0,
                        y: 0.6200000047683716,
                        z: -0.49000000953674316,
                    },
                    using_secondary_action: true,
                },
                selected_hotbar_slot: 4,
            })
        );
    }

    #[test]
    fn right_mouse_press_on_permitted_command_minecart_opens_editor_and_still_interacts() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_command_minecart(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.minecart_command_block_editor_is_active());
        assert!(world.minecart_command_block_editor().is_some());
        assert_eq!(counters.interact_entity_commands_queued, 1);
        let NetCommand::InteractEntity(interact) = rx.try_recv().unwrap() else {
            panic!("expected command-minecart interaction");
        };
        assert_eq!(interact.packet.entity_id, 123);
        assert_eq!(interact.packet.hand, InteractionHand::MainHand);
        assert_eq!(interact.selected_hotbar_slot, 0);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn right_mouse_press_on_minecart_success_swings_main_and_stops() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntityCommand {
                packet: InteractEntity {
                    entity_id: 123,
                    hand: InteractionHand::MainHand,
                    location: ProtocolVec3d {
                        x: 0.0,
                        y: 0.6200000047683716,
                        z: -0.49000000953674316,
                    },
                    using_secondary_action: false,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn right_mouse_press_on_default_interaction_entity_consumes_without_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_INTERACTION_ID);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
            panic!("expected interaction-entity command");
        };
        assert_eq!(command.packet.entity_id, 123);
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_empty_item_frame_uses_populated_offhand() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();
        let player_pose = world.local_player_pose();

        assert!(start_use_item(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            Some(CrosshairTarget::Entity(CrosshairEntityHit {
                entity_id: 123,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 1.62,
                    z: 3.0,
                },
                relative_location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.12,
                    z: 0.0,
                },
            })),
            player_pose,
        ));

        assert_eq!(counters.interact_entity_commands_queued, 2);
        for expected_hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
                panic!("expected item-frame interaction");
            };
            assert_eq!(command.packet.entity_id, 123);
            assert_eq!(command.packet.hand, expected_hand);
        }
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::OffHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_default_mob_pass_tries_both_hands_in_order() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.interact_entity_commands_queued, 2);
        for expected_hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
                panic!("expected default-mob interaction");
            };
            assert_eq!(command.packet.entity_id, 123);
            assert_eq!(command.packet.hand, expected_hand);
        }
    }

    #[test]
    fn spawn_egg_success_on_default_mob_stops_without_client_swing() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 0);
        let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
            panic!("expected spawn-egg mob interaction");
        };
        assert_eq!(command.packet.entity_id, 123);
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
    }

    #[test]
    fn dye_success_on_sheep_swings_main_hand_and_stops_before_offhand() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity_type(123, VANILLA_ENTITY_TYPE_SHEEP_ID);
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:sheep_food".to_string(),
                    entries: Vec::new(),
                }],
            }],
        });
        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_RED_DYE_ID,
            BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID]),
        )]));
        world.set_default_item_dye_color_ids(BTreeMap::from([(
            VANILLA_ITEM_RED_DYE_ID,
            VANILLA_DYE_COLOR_RED_ID,
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.interact_entity_commands_queued, 1);
        let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
            panic!("expected Sheep dye interaction");
        };
        assert_eq!(command.packet.entity_id, 123);
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spectator_right_mouse_press_on_entity_tries_both_hands_in_order() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.interact_entity_commands_queued, 2);
        for expected_hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            let NetCommand::InteractEntity(command) = rx.try_recv().unwrap() else {
                panic!("expected spectator entity interaction");
            };
            assert_eq!(command.packet.entity_id, 123);
            assert_eq!(command.packet.hand, expected_hand);
            assert_eq!(
                command.packet.location,
                ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.49000000953674316,
                }
            );
            assert!(!command.packet.using_secondary_action);
            assert_eq!(command.selected_hotbar_slot, 0);
        }
    }

    #[test]
    fn right_mouse_press_on_entity_outside_world_border_does_not_interact() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_world_border_excluding_crosshair_block(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert_eq!(counters.interact_entity_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_default_pass_block_queues_both_hands() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert_eq!(counters.use_item_on_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
                selected_hotbar_slot: 4,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::OffHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 2,
                },
                selected_hotbar_slot: 4,
            })
        );
    }

    #[test]
    fn right_mouse_press_locks_compass_without_local_sound_and_preserves_drop_swing() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_lodestone();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_COMPASS_ID, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        let locked = world.local_item_in_hand(InteractionHand::MainHand).unwrap();
        assert!(locked
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
        assert_eq!(
            locked.component_patch.lodestone_target,
            Some(LodestoneTargetSummary {
                dimension: "minecraft:overworld".to_string(),
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            })
        );
        assert_eq!(locked.component_patch.lodestone_tracked, Some(true));
        assert!(
            world.client_audio().last_sound.is_none(),
            "CompassItem passes null as the excluded player, so ClientLevel waits for the server sound packet"
        );

        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_lodestone();
        for slot in 0..36 {
            world.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot,
                item: item_stack(VANILLA_ITEM_STONE_ID, 1),
            });
        }
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_COMPASS_ID, 2),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 2);
        assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(world.client_audio().last_sound.is_none());
    }

    #[test]
    fn right_mouse_ignition_plays_only_flint_sound_and_advances_suppressed_randoms() {
        let target = BlockPos { x: 0, y: 1, z: 2 };
        let mut expected_pitch_random = LevelEventSoundRandomState::with_seed(0);
        let expected_pitch = expected_pitch_random.next_float() * 0.4 + 0.8;
        let mut expected_seed_world = WorldStore::new();
        let expected_seed = expected_seed_world.next_sound_seed();

        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_ignitable_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_FLINT_AND_STEEL_ID, 1),
        });
        let mut direct_prediction = world.clone();
        let direct_result = direct_prediction.predict_local_block_use(
            CROSSHAIR_BLOCK_POS,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            false,
            1,
        );
        assert!(
            matches!(
                direct_result,
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
            ),
            "{direct_result:?}"
        );
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert_eq!(audio.positioned_sounds.len(), 1);
        let sound = &audio.positioned_sounds[0];
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:item.flintandsteel.use")
        );
        assert_eq!(sound.source, "block");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 0.5,
                y: 1.5,
                z: 2.5,
            }
        );
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.pitch, expected_pitch);
        assert_eq!(sound.seed, expected_seed);
        assert_eq!(world.client_audio().last_sound.as_ref(), Some(sound));
        assert_eq!(
            world.probe_block(target).unwrap().block_state_id,
            test_block_state_id(
                "minecraft:fire",
                &[
                    ("age", "0"),
                    ("east", "false"),
                    ("north", "false"),
                    ("south", "true"),
                    ("up", "false"),
                    ("west", "false"),
                ],
            )
        );

        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_ignitable_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_FIRE_CHARGE_ID, 2),
        });
        let mut counters = NetCounters::default();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let _ = expected_random.next_float();
        let _ = expected_random.next_float();
        let mut expected_seed_world = WorldStore::new();
        let _discarded_seed = expected_seed_world.next_sound_seed();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(audio.positioned_sounds.is_empty());
        assert!(world.client_audio().last_sound.is_none());
        assert_eq!(random.next_float(), expected_random.next_float());
        assert_eq!(world.next_sound_seed(), expected_next_seed);
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );
    }

    #[test]
    fn default_block_pass_falls_through_to_same_hand_item_use() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        install_consumable(&mut world, 0, VANILLA_ITEM_APPLE_ID, 1.6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItemCommand {
                packet: UseItem {
                    hand: InteractionHand::MainHand,
                    sequence: 2,
                    y_rot: 0.0,
                    x_rot: 0.0,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn right_mouse_press_on_permitted_command_block_opens_editor_and_still_uses_block() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_command_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.command_block_editor_is_active());
        assert!(
            !input
                .command_block_editor_hud_state()
                .unwrap()
                .controls_enabled
        );
        assert!(world.command_block_editor().is_some());
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn right_mouse_press_on_block_outside_world_border_does_not_queue_use_item_on() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        set_world_border_excluding_crosshair_block(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_on_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_block_tries_main_before_populated_offhand() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.use_item_on_commands_queued, 2);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
                selected_hotbar_slot: 0,
            })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::OffHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 2,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn firework_use_on_swings_main_hand_and_stops_before_offhand() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_FIREWORK_ROCKET_ID, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.use_item_on_commands_queued, 1);
        let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
            panic!("expected firework use-on command");
        };
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn bone_meal_full_water_growth_swings_main_hand_without_local_shrink() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: test_block_state_id("minecraft:stone", &[]),
        }));
        let water_pos = BlockPos { x: 0, y: 1, z: 2 };
        let water_state = test_block_state_id("minecraft:water", &[("level", "0")]);
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: water_pos.x,
                y: water_pos.y,
                z: water_pos.z,
            },
            block_state_id: water_state,
        }));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_BONE_MEAL_ID, 3),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_STONE_ID, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(counters.use_item_on_commands_queued, 1);
        let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
            panic!("expected bone-meal use-on command");
        };
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
        assert_eq!(counters.swing_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            3
        );
        assert_eq!(
            world.probe_block(water_pos).unwrap().block_state_id,
            water_state
        );
        assert!(world.local_block_predictions().is_empty());
    }

    #[test]
    fn water_potion_mud_conversion_mutates_once_and_advances_suppressed_sounds() {
        let (tx, mut rx) = mpsc::channel(3);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: CROSSHAIR_BLOCK_POS.x,
                y: CROSSHAIR_BLOCK_POS.y,
                z: CROSSHAIR_BLOCK_POS.z,
            },
            block_state_id: test_block_state_id("minecraft:dirt", &[]),
        }));
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:convertable_to_mud".to_string(),
                    entries: vec![VANILLA_BLOCK_DIRT_ID],
                }],
            }],
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: water_potion_item_stack(1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(VANILLA_ITEM_STONE_ID, 1),
        });
        let mut expected_seed_world = WorldStore::new();
        let _discarded_splash_seed = expected_seed_world.next_sound_seed();
        let _discarded_bottle_empty_seed = expected_seed_world.next_sound_seed();
        let expected_next_seed = expected_seed_world.next_sound_seed();
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut audio = RecordingAudioSink::default();
        let mut audio_events = Some(&mut audio as &mut dyn crate::audio_runtime::AudioEventSink);
        let mut counters = NetCounters::default();

        handle_mouse_input_at_partial_tick_with_effects(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            1.0,
            &mut random,
            &mut audio_events,
        );
        drop(audio_events);

        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.swing_commands_queued, 1);
        let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
            panic!("expected potion use-on command");
        };
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .item_id,
            Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::OffHand)
                .unwrap()
                .item_id,
            Some(VANILLA_ITEM_STONE_ID)
        );
        assert_eq!(
            world
                .probe_block(CROSSHAIR_BLOCK_POS)
                .unwrap()
                .block_state_id,
            test_block_state_id("minecraft:mud", &[])
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert!(audio.positioned_sounds.is_empty());
        assert!(world.client_audio().last_sound.is_none());
        assert_eq!(random.next_float(), expected_random.next_float());
        assert_eq!(world.next_sound_seed(), expected_next_seed);
    }

    #[test]
    fn brush_use_on_starts_main_hand_use_without_swing_or_offhand() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(VANILLA_ITEM_BRUSH_ID, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(world.local_player().interaction.using_item);
        assert_eq!(
            world.local_player().interaction.using_item_hand,
            Some(InteractionHand::MainHand)
        );
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(counters.use_item_commands_queued, 0);
        assert_eq!(counters.swing_commands_queued, 0);
        let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
            panic!("expected brush use-on command");
        };
        assert_eq!(command.packet.hand, InteractionHand::MainHand);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spectator_right_mouse_press_on_block_keeps_main_hand_when_hotbar_is_empty() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert!(input.use_item_held);
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn right_mouse_press_on_block_keeps_main_hand_when_both_hands_have_items() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: item_stack(99, 1),
        });
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOnCommand {
                packet: UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: ProtocolBlockHitResult {
                        pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                        direction: ProtocolDirection::North,
                        cursor_x: 0.0,
                        cursor_y: 0.62,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn held_right_mouse_on_block_repeats_use_item_on_after_vanilla_delay() {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );
        for (expected_hand, expected_sequence) in [
            (InteractionHand::MainHand, 1),
            (InteractionHand::OffHand, 2),
        ] {
            let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
                panic!("expected block-use command");
            };
            assert_eq!(command.packet.hand, expected_hand);
            assert_eq!(command.packet.sequence, expected_sequence);
            assert_eq!(
                command.packet.hit.pos,
                ProtocolBlockPos { x: 0, y: 1, z: 3 }
            );
        }

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            3,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_on_commands_queued, 2);
        assert_eq!(input.use_item_repeat_delay_ticks, 1);

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            1,
        );

        assert_eq!(counters.use_item_on_commands_queued, 4);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        for (expected_hand, expected_sequence) in [
            (InteractionHand::MainHand, 3),
            (InteractionHand::OffHand, 4),
        ] {
            let NetCommand::UseItemOn(command) = rx.try_recv().unwrap() else {
                panic!("expected repeated block-use command");
            };
            assert_eq!(command.packet.hand, expected_hand);
            assert_eq!(command.packet.sequence, expected_sequence);
            assert_eq!(
                command.packet.hit.pos,
                ProtocolBlockPos { x: 0, y: 1, z: 3 }
            );
        }
    }

    #[test]
    fn right_mouse_release_stops_repeated_block_use() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Released,
        );
        assert!(!input.use_item_held);
        assert_eq!(input.use_item_repeat_delay_ticks, 0);

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            USE_ITEM_REPEAT_DELAY_TICKS.into(),
        );

        assert_eq!(counters.use_item_on_commands_queued, 2);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_right_mouse_without_target_does_not_repeat_while_item_is_in_use() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            y_rot: 45.0,
            x_rot: -20.0,
            ..LocalPlayerPoseState::default()
        });
        install_consumable(&mut world, 0, 42, 1.6);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );
        assert!(world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_commands_queued, 1);
        assert!(matches!(rx.try_recv().unwrap(), NetCommand::UseItem(_)));

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            USE_ITEM_REPEAT_DELAY_TICKS.into(),
        );

        assert_eq!(counters.use_item_commands_queued, 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_ender_dragon_part_queues_interact_with_part_id() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_ender_dragon();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
        );

        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntityCommand {
                packet: InteractEntity {
                    entity_id: 101,
                    hand: InteractionHand::MainHand,
                    location: ProtocolVec3d {
                        x: 0.0,
                        y: 0.6200000047683716,
                        z: -0.5,
                    },
                    using_secondary_action: false,
                },
                selected_hotbar_slot: 0,
            })
        );
    }

    #[test]
    fn control_middle_mouse_press_on_block_queues_pick_block_with_data() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.control_left_down = true;
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert_eq!(counters.pick_item_from_block_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromBlock(PickItemFromBlock {
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                include_data: true,
            })
        );
    }

    #[test]
    fn control_middle_mouse_press_on_entity_queues_pick_entity_with_data() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.control_left_down = true;
        let mut world = world_with_crosshair_entity(123);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert_eq!(counters.pick_item_from_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromEntity(PickItemFromEntity {
                entity_id: 123,
                include_data: true,
            })
        );
    }

    #[test]
    fn sprint_middle_mouse_press_does_not_include_pick_data() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sprint = true;
        let mut world = world_with_crosshair_block();
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert_eq!(counters.pick_item_from_block_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PickItemFromBlock(PickItemFromBlock {
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                include_data: false,
            })
        );
    }

    #[test]
    fn spectator_middle_mouse_opens_menu_instead_of_picking_target() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert!(input.spectator_menu_is_active());
        assert_eq!(counters.pick_item_from_entity_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spectator_middle_mouse_does_nothing_while_using_item() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_entity(123);
        set_local_spectator(&mut world);
        world.set_local_using_item(true);
        let mut counters = NetCounters::default();

        handle_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Middle,
            ElementState::Pressed,
        );

        assert!(!input.spectator_menu_is_active());
        assert_eq!(counters.pick_item_from_entity_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn mouse_wheel_selects_hotbar_slot_updates_world_and_queues_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 3);
        assert_eq!(world.counters().held_slot_packets, 0);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
    }

    #[test]
    fn spectator_mouse_wheel_does_not_select_hotbar_or_queue_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        set_local_spectator(&mut world);
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spectator_mouse_wheel_adjusts_local_flying_speed_without_hotbar_command() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        set_local_spectator(&mut world);
        set_flying_abilities(&mut world, 0.05);
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.055);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spectator_mouse_wheel_selects_active_menu_without_changing_flying_speed() {
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        set_local_spectator(&mut world);
        set_flying_abilities(&mut world, 0.05);
        let mut counters = NetCounters::default();
        assert!(input.handle_spectator_hotbar_action(&world, &mut counters, &None));

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &None,
            MouseScrollDelta::LineDelta(0.0, -1.0),
        );

        assert_eq!(input.spectator_menu_selected_slot(), Some(8));
        assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.05);
    }

    #[test]
    fn spectator_mouse_wheel_clamps_speed_and_ignores_horizontal_steps() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        set_local_spectator(&mut world);
        set_flying_abilities(&mut world, 0.199);
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );
        assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.2);

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(1.0, 0.0),
        );

        assert_eq!(world.local_player().abilities.unwrap().flying_speed, 0.2);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn mouse_wheel_wraps_hotbar_selection_like_vanilla() {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );
        assert_eq!(world.local_player().selected_hotbar_slot, 8);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(8));

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, -1.0),
        );
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        assert_eq!(counters.held_slot_commands_queued, 2);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(0));
    }

    #[test]
    fn unfocused_mouse_wheel_does_not_select_or_queue() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(false);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn sign_editor_mouse_wheel_does_not_select_or_queue() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_empty_sign(BlockPos { x: 1, y: 2, z: 3 });
        assert!(world.set_local_selected_hotbar_slot(4));
        world.apply_open_sign_editor(OpenSignEditor {
            pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
            is_front_text: true,
        });
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 1.0),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    fn world_with_empty_sign(pos: BlockPos) -> WorldStore {
        let mut world = WorldStore::new();
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos {
                x: pos.x.div_euclid(16),
                z: pos.z.div_euclid(16),
            },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: vec![BlockEntityRecord {
                local_x: pos.x.rem_euclid(16) as u8,
                y: i16::try_from(pos.y).unwrap(),
                local_z: pos.z.rem_euclid(16) as u8,
                type_id: bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: vec![0],
                nbt: None,
                sign_text: Some(SignBlockEntityTextState::default()),
                command_block: None,
                structure_block: None,
                jigsaw_block: None,
                test_block: None,
                test_instance_block: None,
                vault_shared_data: None,
                decorated_pot_sherds: None,
                banner_patterns: None,
                end_gateway: None,
                spawner: None,
                note_block_sound: None,
                skull_profile: None,
            }],
            light: LightData::default(),
        });
        world
    }

    #[test]
    fn fractional_mouse_wheel_accumulates_before_selecting() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(4));
        let mut counters = NetCounters::default();

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 0.5),
        );
        assert_eq!(world.local_player().selected_hotbar_slot, 4);
        assert!(rx.try_recv().is_err());

        handle_mouse_wheel(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseScrollDelta::LineDelta(0.0, 0.5),
        );

        assert_eq!(world.local_player().selected_hotbar_slot, 3);
        assert_eq!(counters.held_slot_commands_queued, 1);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(3));
    }

    fn world_with_crosshair_entity(entity_id: i32) -> WorldStore {
        world_with_crosshair_entity_type_at(
            entity_id,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        )
    }

    fn world_with_crosshair_entity_type(entity_id: i32, entity_type_id: i32) -> WorldStore {
        world_with_crosshair_entity_type_at(entity_id, entity_type_id, [0.0, 1.0, 3.0])
    }

    fn local_attack_world_with_player_and_target(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.apply_add_entity(AddEntity {
            id: 42,
            uuid: Uuid::from_u128(42),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_MINECART_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }

    fn world_with_crosshair_command_minecart(entity_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        assert!(world.apply_entity_event(ProtocolEntityEvent {
            entity_id: 42,
            event_id: 26,
        }));
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }

    fn world_with_crosshair_entity_at(entity_id: i32, position: [f64; 3]) -> WorldStore {
        world_with_crosshair_entity_type_at(entity_id, VANILLA_ENTITY_TYPE_MINECART_ID, position)
    }

    fn world_with_crosshair_entity_type_at(
        entity_id: i32,
        entity_type_id: i32,
        position: [f64; 3],
    ) -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id,
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }

    fn world_with_crosshair_block() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    bbb_world::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
        }));
        world
    }

    fn world_with_crosshair_lodestone() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..24)
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        let lodestone_state =
            bbb_world::vanilla_block_state_id("minecraft:lodestone", &BTreeMap::new()).unwrap();
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: lodestone_state,
        }));
        world
    }

    fn world_with_crosshair_ignitable_block() -> WorldStore {
        let mut world = world_with_crosshair_lodestone();
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: -1, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..24)
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: test_block_state_id("minecraft:oak_planks", &[]),
        }));
        world
    }

    fn test_block_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        bbb_world::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("missing Vanilla block state {name} {properties:?}"))
    }

    fn world_with_crosshair_command_block() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        assert!(world.apply_entity_event(ProtocolEntityEvent {
            entity_id: 42,
            event_id: 26,
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..24)
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        let command_block_state = bbb_world::vanilla_block_state_id(
            "minecraft:command_block",
            &BTreeMap::from([
                ("conditional".to_string(), "false".to_string()),
                ("facing".to_string(), "north".to_string()),
            ]),
        )
        .unwrap();
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
            block_state_id: command_block_state,
        }));
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
                raw_nbt: vec![10, 0],
            })
            .unwrap());
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn water_potion_item_stack(count: i32) -> ProtocolItemStackSummary {
        let mut stack = item_stack(VANILLA_ITEM_POTION_ID, count);
        stack.component_patch.added = 1;
        stack
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID);
        stack.component_patch.potion_id = Some(0);
        stack
    }

    fn install_consumable(world: &mut WorldStore, slot: i32, item_id: i32, consume_seconds: f32) {
        world.set_default_item_component_type_ids(BTreeMap::from([(
            item_id,
            BTreeSet::from([VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID]),
        )]));
        world.set_default_item_consumables(BTreeMap::from([(
            item_id,
            ConsumableSummary {
                consume_seconds,
                ..ConsumableSummary::default()
            },
        )]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot,
            item: item_stack(item_id, 1),
        });
    }

    fn item_stack_with_attack_range(
        item_id: i32,
        count: i32,
        min_reach: f32,
        max_reach: f32,
    ) -> ProtocolItemStackSummary {
        let mut stack = item_stack(item_id, count);
        stack.component_patch.added = 1;
        stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID];
        stack.component_patch.attack_range = Some(AttackRangeSummary {
            min_reach,
            max_reach,
            min_creative_reach: min_reach,
            max_creative_reach: max_reach,
            hitbox_margin: 0.0,
            mob_factor: 1.0,
        });
        stack
    }

    fn set_selected_piercing_weapon(world: &mut WorldStore, item_id: i32) {
        world.set_default_piercing_weapon_item_ids(BTreeSet::from([item_id]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(item_id, 1),
        });
    }

    fn set_selected_can_break(world: &mut WorldStore, block: &str) {
        let block_id = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block)
            .unwrap()
            .id;
        let mut stack = item_stack(1, 1);
        stack.component_patch.added = 1;
        stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID];
        stack.component_patch.can_break = Some(AdventureModePredicateSummary {
            unknown: false,
            block_registry_ids: vec![block_id],
            unresolved_block_tag_count: 0,
        });
        stack.component_patch.can_break_persistent = Some(AdventureModePredicateDataSummary {
            predicates: vec![BlockPredicateSummary {
                blocks: Some(BlockHolderSetSummary {
                    registry_ids: vec![block_id],
                    tag: None,
                }),
                ..BlockPredicateSummary::default()
            }],
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: stack,
        });
    }

    fn world_with_ender_dragon() -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        world.apply_add_entity(AddEntity {
            id: 100,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 2.0,
                z: 9.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.advance_entity_client_animations(1);
        world
    }
}
