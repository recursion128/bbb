use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    Direction as ProtocolDirection, InteractionHand, PlayerActionKind, SpectateEntity,
};
use bbb_world::{
    BlockPos as WorldBlockPos, ItemAttackRange, LocalDestroyBlockFinished, LocalPlayerPoseState,
    WorldStore,
};
use tokio::sync::mpsc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

use crate::audio_runtime::AudioEventSink;
use crate::camera_pose::camera_pose_from_world;
use crate::crosshair::{
    crosshair_target_from_camera_at_partial_tick, CrosshairBlockHit, CrosshairEntityHit,
    CrosshairTarget,
};

use super::{
    commands::{
        hotbar_slot_for_scroll, queue_attack_entity_command, queue_interact_entity_command,
        queue_pick_item_from_block_command, queue_pick_item_from_entity_command,
        queue_player_action_command, queue_spectate_entity_command, queue_swing_command,
        queue_use_item_command, queue_use_item_on_command, queue_zero_pos_player_action_command,
        select_hotbar_slot,
    },
    ClientInputState,
};

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

pub(crate) fn handle_mouse_input_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    entity_partial_tick: f32,
) {
    if !input.focused || input.sign_editor_is_active_or_pending(world) {
        return;
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
            if local_player_attack_is_blocked(world) {
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
            if world.local_selected_main_hand_has_piercing_weapon() {
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::Stab,
                );
                queue_swing_command(counters, net_commands, InteractionHand::MainHand);
                return;
            }
            input.destroy_block_held = true;
            match camera_target {
                Some(CrosshairTarget::Entity(hit)) => {
                    if selected_attack_range_allows_entity_hit(world, player_pose, hit) {
                        queue_attack_entity_command(counters, net_commands, hit.entity_id);
                    }
                }
                Some(CrosshairTarget::Block(hit)) => {
                    if block_target_within_world_border(world, hit.pos) {
                        start_destroy_block(counters, world, net_commands, hit);
                    }
                }
                None => {}
            }
            queue_swing_command(counters, net_commands, InteractionHand::MainHand);
        }
        (MouseButton::Left, ElementState::Released) => {
            input.destroy_block_held = false;
            if world.local_selected_main_hand_has_piercing_weapon() {
                return;
            }
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        }
        (MouseButton::Right, ElementState::Pressed) => {
            input.use_item_held = true;
            start_use_item(
                input,
                world,
                counters,
                net_commands,
                camera_target,
                player_pose,
            );
        }
        (MouseButton::Right, ElementState::Released) => {
            input.use_item_held = false;
            input.use_item_repeat_delay_ticks = 0;
            if world.take_local_using_item() {
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::ReleaseUseItem,
                );
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

pub(crate) fn advance_using_item_at_partial_tick(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    entity_partial_tick: f32,
    use_ticks: u32,
) {
    if !input.focused
        || input.sign_editor_is_active_or_pending(world)
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
        if !start_use_item(
            input,
            world,
            counters,
            net_commands,
            camera_target,
            player_pose,
        ) || ticks_remaining == 0
        {
            return;
        }
    }
}

fn start_use_item(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    camera_target: Option<CrosshairTarget>,
    player_pose: Option<bbb_world::LocalPlayerPoseState>,
) -> bool {
    if world.local_player().interaction.using_item
        || world.local_player().interaction.destroying_block.is_some()
    {
        return false;
    }
    if world.local_player_is_spectator() && camera_target.is_none() {
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

    input.use_item_repeat_delay_ticks = USE_ITEM_REPEAT_DELAY_TICKS;
    match camera_target {
        Some(CrosshairTarget::Entity(hit)) => {
            queue_interact_entity_command(
                counters,
                net_commands,
                hit.entity_id,
                InteractionHand::MainHand,
                hit.relative_location,
                input.sneak,
            );
        }
        Some(CrosshairTarget::Block(hit)) => {
            let sequence = world.next_local_prediction_sequence();
            let hand = block_use_hand(world);
            queue_use_item_on_command(counters, net_commands, hand, hit, sequence);
        }
        None => {
            if let Some(pose) = player_pose {
                let sequence = world.next_local_prediction_sequence();
                let hand = item_use_hand(world);
                let using_item = queue_use_item_command(
                    counters,
                    net_commands,
                    hand,
                    pose.y_rot,
                    pose.x_rot,
                    sequence,
                );
                world.set_local_using_item(using_item);
            }
        }
    }
    true
}

fn item_use_hand(world: &WorldStore) -> InteractionHand {
    if world.local_item_use_prefers_offhand() {
        InteractionHand::OffHand
    } else {
        InteractionHand::MainHand
    }
}

fn block_use_hand(world: &WorldStore) -> InteractionHand {
    if world.local_player_is_spectator() {
        InteractionHand::MainHand
    } else {
        item_use_hand(world)
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
    if !input.focused || input.sign_editor_is_active_or_pending(world) || !input.destroy_block_held
    {
        return;
    }
    if local_player_attack_is_blocked(world) {
        return;
    }
    if world.local_selected_main_hand_has_piercing_weapon() {
        return;
    }
    if world.tick_local_destroy_delay() {
        return;
    }

    let camera_target = crosshair_target_from_camera_at_partial_tick(
        world,
        camera_pose_from_world(world),
        entity_partial_tick,
    );
    match camera_target {
        Some(CrosshairTarget::Block(hit)) if block_target_within_world_border(world, hit.pos) => {
            continue_destroy_block(
                counters,
                world,
                net_commands,
                &mut audio_events,
                hit,
                destroy_ticks,
            )
        }
        Some(CrosshairTarget::Block(_)) => {}
        _ => {
            abort_destroy_block(counters, world, net_commands, ProtocolDirection::Down);
        }
    }
}

fn continue_destroy_block(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    hit: CrosshairBlockHit,
    destroy_ticks: u32,
) {
    if world.local_player().interaction.destroying_block == Some(hit.pos)
        && world.local_destroying_block_matches_current_item()
    {
        world.update_local_destroying_block_face(hit.face);
        for _ in 0..destroy_ticks {
            maybe_play_destroy_hit_sound(audio_events, world, hit.pos);
            if let Some(finished) = world.advance_local_destroying_block_tick() {
                stop_destroy_block(counters, net_commands, finished);
                break;
            }
        }
        return;
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
    start_destroy_block(counters, world, net_commands, hit);
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
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    hit: CrosshairBlockHit,
) {
    if !block_target_within_world_border(world, hit.pos) {
        return;
    }

    let sequence = world.next_local_prediction_sequence();
    queue_player_action_command(
        counters,
        net_commands,
        PlayerActionKind::StartDestroyBlock,
        hit.pos,
        hit.face,
        sequence,
    );
    if local_player_instabuild(world) || world.local_destroy_block_is_immediate(hit.pos) {
        world.predict_local_destroy_block(hit.pos, sequence);
        world.set_local_destroy_delay_ticks(5);
        return;
    }
    world.set_local_destroying_block_hit(hit.pos, hit.face);
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

fn local_player_attack_is_blocked(world: &WorldStore) -> bool {
    world.local_player().interaction.using_item
}

pub(crate) fn handle_mouse_wheel(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    delta: MouseScrollDelta,
) {
    if !input.focused || input.sign_editor_is_active_or_pending(world) {
        return;
    }
    let Some(wheel) = wheel_steps_from_scroll(input, delta) else {
        return;
    };
    if world.local_player_is_spectator() {
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
    use bbb_protocol::packets::{
        AddEntity, AttackEntity, AttackRangeSummary, BlockHitResult as ProtocolBlockHitResult,
        BlockPos as ProtocolBlockPos, BlockUpdate, GameEvent as ProtocolGameEvent,
        InitializeBorder, InteractEntity, ItemStackSummary as ProtocolItemStackSummary,
        OpenSignEditor, PickItemFromBlock, PickItemFromEntity, PlayerAbilities, PlayerAction,
        SetPlayerInventory as ProtocolSetPlayerInventory, UseItem, UseItemOn,
        Vec3d as ProtocolVec3d,
    };
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldBlockSoundProfile, WorldDimension,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use uuid::Uuid;

    const CROSSHAIR_BLOCK_POS: BlockPos = BlockPos { x: 0, y: 1, z: 3 };
    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;
    const VANILLA_GRASS_BLOCK_STATE_ID: i32 = 9;
    const VANILLA_ENTITY_TYPE_AXOLOTL_ID: i32 = 7;
    const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;
    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;
    const VANILLA_ATTACK_RANGE_COMPONENT_ID: i32 = 30;
    const VANILLA_PLAYER_OFFHAND_SLOT: i32 = 40;
    const VANILLA_WORLD_BORDER_ABSOLUTE_MAX_SIZE: i32 = 29_999_984;

    #[derive(Default)]
    struct RecordingAudioSink {
        positioned_sounds: Vec<bbb_world::SoundEventState>,
    }

    impl AudioEventSink for RecordingAudioSink {
        fn counters(&self) -> bbb_control::AudioCounters {
            bbb_control::AudioCounters::default()
        }

        fn set_sound_event_registry(&mut self, _registry: bbb_audio::SoundEventRegistry) {}

        fn play_local_sound(&mut self, _state: &bbb_world::LocalSoundEventState) {}

        fn play_positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
            self.positioned_sounds.push(state.clone());
        }

        fn play_entity_sound(
            &mut self,
            _state: &bbb_world::SoundEntityEventState,
            _position: Option<[f64; 3]>,
        ) {
        }

        fn stop_sound(&mut self, _state: &bbb_world::StopSoundEventState) {}

        fn tick_entity_sound_positions(
            &mut self,
            _command: bbb_audio::TickEntitySoundPositionsCommand,
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
        let mut world = WorldStore::new();
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
            NetCommand::UseItem(UseItem {
                hand: InteractionHand::MainHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
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
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_OFFHAND_SLOT,
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
        assert_eq!(counters.use_item_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItem(UseItem {
                hand: InteractionHand::OffHand,
                sequence: 1,
                y_rot: 45.0,
                x_rot: -20.0,
            })
        );
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
        assert_eq!(input.use_item_repeat_delay_ticks, 0);
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
            NetCommand::AttackEntity(AttackEntity { entity_id: 123 })
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
        let (tx, mut rx) = mpsc::channel(2);
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
            VANILLA_AIR_BLOCK_STATE_ID
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 1);
        assert_eq!(counters.player_action_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlayerAction(PlayerAction {
                action: PlayerActionKind::StopDestroyBlock,
                pos: ProtocolBlockPos { x: 0, y: 1, z: 3 },
                direction: ProtocolDirection::North,
                sequence: 1,
            })
        );
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
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn held_left_mouse_same_target_emits_vanilla_block_hit_sound_every_four_ticks() {
        let (tx, mut rx) = mpsc::channel(1);
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
            VANILLA_AIR_BLOCK_STATE_ID
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
            NetCommand::AttackEntity(AttackEntity { entity_id: 123 })
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::Swing(InteractionHand::MainHand)
        );
    }

    #[test]
    fn right_mouse_press_on_entity_queues_interact_with_relative_hit_location() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.sneak = true;
        let mut world = world_with_crosshair_entity(123);
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
        assert_eq!(counters.interact_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::InteractEntity(InteractEntity {
                entity_id: 123,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.49000000953674316,
                },
                using_secondary_action: true,
            })
        );
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
        assert_eq!(input.use_item_repeat_delay_ticks, 0);
        assert_eq!(counters.interact_entity_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_block_queues_use_item_on() {
        let (tx, mut rx) = mpsc::channel(1);
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

        assert!(!world.local_player().interaction.using_item);
        assert!(input.use_item_held);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
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
        assert_eq!(input.use_item_repeat_delay_ticks, 0);
        assert!(!world.local_player().interaction.using_item);
        assert_eq!(counters.use_item_on_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn right_mouse_press_on_block_uses_offhand_when_selected_hotbar_slot_is_empty() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_OFFHAND_SLOT,
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
            NetCommand::UseItemOn(UseItemOn {
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
                sequence: 1,
            })
        );
    }

    #[test]
    fn spectator_right_mouse_press_on_block_keeps_main_hand_when_hotbar_is_empty() {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut world = world_with_crosshair_block();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: VANILLA_PLAYER_OFFHAND_SLOT,
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
            NetCommand::UseItemOn(UseItemOn {
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
            slot: VANILLA_PLAYER_OFFHAND_SLOT,
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
            NetCommand::UseItemOn(UseItemOn {
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
            })
        );
    }

    #[test]
    fn held_right_mouse_on_block_repeats_use_item_on_after_vanilla_delay() {
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
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
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
            })
        );

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            3,
        );
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.use_item_on_commands_queued, 1);
        assert_eq!(input.use_item_repeat_delay_ticks, 1);

        advance_using_item_at_partial_tick(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            1.0,
            1,
        );

        assert_eq!(counters.use_item_on_commands_queued, 2);
        assert_eq!(
            input.use_item_repeat_delay_ticks,
            USE_ITEM_REPEAT_DELAY_TICKS
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::UseItemOn(UseItemOn {
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
                sequence: 2,
            })
        );
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

        assert_eq!(counters.use_item_on_commands_queued, 1);
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
            NetCommand::InteractEntity(InteractEntity {
                entity_id: 101,
                hand: InteractionHand::MainHand,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.6200000047683716,
                    z: -0.5,
                },
                using_secondary_action: false,
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
        let mut world = WorldStore::new();
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
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState::default());
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
                    VANILLA_AIR_BLOCK_STATE_ID,
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

    fn item_stack_with_attack_range(
        item_id: i32,
        count: i32,
        min_reach: f32,
        max_reach: f32,
    ) -> ProtocolItemStackSummary {
        let mut stack = item_stack(item_id, count);
        stack.component_patch.added = 1;
        stack.component_patch.added_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
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
