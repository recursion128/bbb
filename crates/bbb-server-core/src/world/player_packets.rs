//! The play-phase packets a player's own state becomes, and the position,
//! rotation and audible-range reads they are built from.

mod ground_tick;
mod movement;

use crate::chunk_sender::ChunkKey;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, PlayerInfoEntry, PlayerInfoUpdate, PlayerRotation, SoundEvent, SoundEventHolder,
    UseItemOn, Vec3d,
};
use bbb_protocol::player_info_actions::PlayerInfoAction;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::stats::custom_stat;

use super::player_movement::PlayerMoveProbe;
use super::{
    ClientboundPlayPacket, ConnectionId, PlayerState, ServerWorld, WorldEvent, DEFAULT_SOUND_RANGE,
    HORIZONTAL_LIMIT, VERTICAL_LIMIT,
};

/// `:1133`: `movedDist > 0.0625` is the squared quarter-block band a client
/// may differ from the server's own solve by.
const MOVED_WRONGLY_DISTANCE: f64 = 0.0625;

/// `SoundEvent.getRange(volume)` (`SoundEvent.java:40-42`): a sound's own fixed
/// range when it declares one, and otherwise sixteen blocks scaled by any
/// volume above one.
///
/// Only an inline `Holder` can carry a fixed range here: every `SoundEvents`
/// registration goes through `SoundEvent.createVariableRangeEvent`
/// (`SoundEvents.java:1860-1866`), so a sound sent by registry reference never
/// has one.
pub(super) fn sound_broadcast_range(sound: &SoundEvent) -> f64 {
    let fixed_range = match &sound.sound {
        SoundEventHolder::Reference { .. } => None,
        SoundEventHolder::Direct { fixed_range, .. } => *fixed_range,
    };
    f64::from(fixed_range.unwrap_or(if sound.volume > 1.0 {
        DEFAULT_SOUND_RANGE * sound.volume
    } else {
        DEFAULT_SOUND_RANGE
    }))
}

/// `Entity.blockPosition` through `BlockPos.containing` (`Mth.floor`).
pub(super) fn block_position(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

/// `Entity.chunkPosition` (Entity.java:3646-3647), which tracks the block
/// position through `SectionPos.blockToSectionCoord` (an arithmetic shift, not
/// a division).
pub(super) fn player_chunk(position: Vec3d) -> ChunkKey {
    let pos = block_position(position);
    (pos.x >> 4, pos.z >> 4)
}

pub(super) fn send(connection_id: ConnectionId, packet: ClientboundPlayPacket) -> WorldEvent {
    WorldEvent::Send {
        connection_id,
        packet,
    }
}

pub(super) fn broadcast_to(
    recipients: impl IntoIterator<Item = ConnectionId>,
    packet: ClientboundPlayPacket,
    events: &mut Vec<WorldEvent>,
) {
    let mut iter = recipients.into_iter();
    let Some(first) = iter.next() else {
        return;
    };
    let Some(second) = iter.next() else {
        events.push(send(first, packet));
        return;
    };
    let mut all = Vec::with_capacity(2 + iter.size_hint().0);
    all.push(first);
    all.push(second);
    all.extend(iter);
    events.push(WorldEvent::Broadcast {
        recipients: all,
        packet,
    });
}

/// One full eight-action "player initializing" entry
/// (`ClientboundPlayerInfoUpdatePacket.createPlayerInitializing`,
/// PlayerList.java:207): profile, no chat session, the session's own game mode,
/// listed, its keep-alive latency, no display name, default list order, and the
/// hat flag the client asked for.
pub(super) fn initializing_player_info_entry(player: &PlayerState) -> PlayerInfoEntry {
    PlayerInfoEntry {
        profile_id: player.profile.uuid,
        profile: Some(player.profile.clone()),
        listed: true,
        latency: player.cold.latency,
        game_mode: player.game_type,
        display_name: None,
        show_hat: player.cold.client_information.model_parts().hat,
        list_order: 0,
        chat_session: None,
    }
}

pub(super) fn initializing_player_info(entries: Vec<PlayerInfoEntry>) -> PlayerInfoUpdate {
    PlayerInfoUpdate {
        actions: vec![
            PlayerInfoAction::AddPlayer,
            PlayerInfoAction::InitializeChat,
            PlayerInfoAction::UpdateGameMode,
            PlayerInfoAction::UpdateListed,
            PlayerInfoAction::UpdateLatency,
            PlayerInfoAction::UpdateDisplayName,
            PlayerInfoAction::UpdateListOrder,
            PlayerInfoAction::UpdateHat,
        ],
        entries,
    }
}

pub(super) fn hit_location_is_finite(update: UseItemOn) -> bool {
    update.hit.cursor_x.is_finite()
        && update.hit.cursor_y.is_finite()
        && update.hit.cursor_z.is_finite()
}

pub(super) fn movement_is_invalid(
    position: Option<Vec3d>,
    rotation: Option<PlayerRotation>,
) -> bool {
    // ServerGamePacketListenerImpl.containsInvalidValues rejects NaN positions
    // and non-finite rotations; infinities are subsequently clamped by the
    // separate horizontal/vertical coordinate limits.
    position
        .is_some_and(|position| position.x.is_nan() || position.y.is_nan() || position.z.is_nan())
        || rotation
            .is_some_and(|rotation| !rotation.x_rot.is_finite() || !rotation.y_rot.is_finite())
}

/// What one `handleMovePlayer` pass decided
/// (`ServerGamePacketListenerImpl.java:1053-1191`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MovementOutcome {
    /// The packet was accepted as a move, which is what tells the caller to
    /// run [`check_movement_statistics`] over the block the accepted position
    /// now stands in.
    Accepted,
    /// `:1103-1106`: the move was too quick for the tick rate, so the position
    /// is unchanged and the caller has to send the client back to it.
    TooQuickly,
    /// `:1179-1181`: the collision solve puts the client's target somewhere
    /// else, so the caller sends it back to where the packet started with the
    /// rotation it asked for.
    MovedWrongly(PlayerRotation),
    SleepingMoved(PlayerRotation),
    /// `:1067-1068`, `:1073-1075`: the packet carried no position this server
    /// may accept, or the session is riding something that owns it.
    Ignored,
}

/// The game-rule and tick-rate inputs `shouldCheckPlayerMovement`
/// (`:1193-1204`) and the packet-rate arm (`:1091-1109`) read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MovementValidationFacts {
    /// `GameRules.PLAYER_MOVEMENT_CHECK`.
    pub movement_check: bool,
    /// `GameRules.ELYTRA_MOVEMENT_CHECK`.
    pub elytra_movement_check: bool,
    /// `level.tickRateManager().runsNormally()`.
    pub runs_normally: bool,
    /// `player.isPassenger()`.
    pub is_passenger: bool,
}

impl MovementValidationFacts {
    /// `shouldCheckPlayerMovement` (`:1193-1204`): the dedicated server is
    /// never the singleplayer owner, and `isChangingDimension` is on the
    /// player.
    fn checks(self, fall_flying: bool, is_changing_dimension: bool) -> bool {
        self.movement_check
            && (!fall_flying || self.elytra_movement_check)
            && !is_changing_dimension
    }
}

/// `clampHorizontal`/`clampVertical` (`:1070-1072`) over a packet's position.
pub(super) fn clamped_target(position: Vec3d) -> Vec3d {
    Vec3d {
        x: position
            .x
            .clamp(-f64::from(HORIZONTAL_LIMIT), f64::from(HORIZONTAL_LIMIT)),
        y: position.y.clamp(-VERTICAL_LIMIT, VERTICAL_LIMIT),
        z: position
            .z
            .clamp(-f64::from(HORIZONTAL_LIMIT), f64::from(HORIZONTAL_LIMIT)),
    }
}

/// The arms `handleMovePlayer` reaches before `player.move` (`:1064-1109`):
/// the loaded gate, the outstanding-teleport and passenger arms, and the
/// packet-rate and tick-budget refusal.
///
/// `Some` is a decision that needs no collision probe; `None` means the packet
/// goes on to the move, which is why the caller computes the probe only then.
pub(super) fn movement_precheck(
    player: &mut PlayerState,
    movement: &bbb_protocol::packets::MovePlayer,
    facts: MovementValidationFacts,
) -> Option<MovementOutcome> {
    // `ServerGamePacketListenerImpl.java:1060-1062`: reset the baseline on the
    // very first tick before any `tick_players` reset has run.
    if player.tick_count == 0 {
        let position = player.position();
        player.first_good = position;
        player.last_good = position;
    }
    if !player.client_loaded() {
        return Some(MovementOutcome::Ignored);
    }
    let rotation = movement.rotation.map(|rotation| PlayerRotation {
        y_rot: wrap_degrees(rotation.y_rot),
        x_rot: wrap_degrees(rotation.x_rot),
    });
    if player.awaiting_teleport.is_some() {
        // `:1067-1068`: while the client owes an answer to a teleport, only
        // `absSnapRotationTo` runs.
        if let Some(rotation) = rotation {
            player.set_rotation(rotation);
        }
        return Some(MovementOutcome::Ignored);
    }
    if facts.is_passenger {
        // `:1073-1075`: a rider's own position report is ignored -- the vehicle
        // owns where it is -- and only the rotation lands. The caller's
        // chunk-source move runs for every packet.
        if let Some(rotation) = rotation {
            player.set_rotation(rotation);
        }
        return Some(MovementOutcome::Ignored);
    }
    // A rotation-only packet falls back to the position the session is already
    // at (`packet.getX(this.player.getX())`, `:1070-1072`), so the rate and
    // moved-too-quickly arms below see it too.
    let target = movement.position.map_or(player.position(), clamped_target);
    // Hard anti-DoS cap on single-packet displacement and swept collision volume
    // even when speed checks are disabled (e.g. frozen tick or player_movement_check=false).
    const MAX_UNCHECKED_MOVE_AXIS_DELTA: f64 = 512.0;
    const MAX_SWEEP_COLLISION_BLOCKS: u64 = 65_536;
    let dx = (target.x - player.position().x).abs();
    let dy = (target.y - player.position().y).abs();
    let dz = (target.z - player.position().z).abs();
    let swept_blocks = (dx.ceil() as u64 + 2)
        .saturating_mul(dy.ceil() as u64 + 3)
        .saturating_mul(dz.ceil() as u64 + 2);
    if dx > MAX_UNCHECKED_MOVE_AXIS_DELTA
        || dy > MAX_UNCHECKED_MOVE_AXIS_DELTA
        || dz > MAX_UNCHECKED_MOVE_AXIS_DELTA
        || swept_blocks > MAX_SWEEP_COLLISION_BLOCKS
    {
        tracing::warn!(
            player = %player.profile.name,
            dx,
            dy,
            dz,
            "player movement packet exceeded swept collision budget"
        );
        return Some(MovementOutcome::TooQuickly);
    }
    // ServerGamePacketListenerImpl.java:1085-1088: sleep ignores movement before rate accounting.
    if player.cold.sleeping_pos.is_some() {
        return Some(if squared_distance(target, player.first_good) > 1.0 {
            MovementOutcome::SleepingMoved(rotation.unwrap_or(player.rotation()))
        } else {
            MovementOutcome::Ignored
        });
    }
    // `:1091-1109`: the packet-rate and moved-too-quickly arms only run while
    // the tick rate is normal.
    if facts.runs_normally {
        player.received_move_packets = player.received_move_packets.wrapping_add(1);
        let mut delta_packets = player
            .received_move_packets
            .wrapping_sub(player.known_move_packets);
        if delta_packets <= 0 || delta_packets > 5 {
            if delta_packets > 5 {
                tracing::debug!(
                    player = %player.profile.name,
                    delta_packets,
                    "player is sending move packets too frequently"
                );
            }
            delta_packets = 1;
        }
        let fall_flying = player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING);
        if facts.checks(fall_flying, player.is_changing_dimension) {
            let meters_per_tick = if fall_flying { 300.0 } else { 100.0 };
            let moved = squared_distance(target, player.first_good);
            // `:1083-1084`: the distance the server's own simulation expected;
            // guard against non-finite velocity bypassing the check.
            let raw_expected = player.delta_movement().length_sqr();
            let expected = if raw_expected.is_finite() {
                raw_expected
            } else {
                0.0
            };
            if moved - expected > meters_per_tick * f64::from(delta_packets) {
                tracing::warn!(
                    player = %player.profile.name,
                    dx = target.x - player.first_good.x,
                    dy = target.y - player.first_good.y,
                    dz = target.z - player.first_good.z,
                    "player moved too quickly"
                );
                return Some(MovementOutcome::TooQuickly);
            }
        }
    }
    None
}

/// The move itself: the collision probe's verdict (`:1111-1145`) and the state
/// an accepted packet leaves behind (`:1146-1178`).
pub(super) fn apply_movement(
    player: &mut PlayerState,
    scoreboard: &mut super::server_scoreboard::ServerScoreboard,
    movement: bbb_protocol::packets::MovePlayer,
    probe: PlayerMoveProbe,
) -> MovementOutcome {
    let rotation = movement.rotation.map(|rotation| PlayerRotation {
        y_rot: wrap_degrees(rotation.y_rot),
        x_rot: wrap_degrees(rotation.x_rot),
    });
    let start = probe.packet_start.unwrap_or(player.position());
    let target = movement.position.map_or(start, clamped_target);
    if !probe.prefix_applied
        && apply_player_move_prefix(player, scoreboard, movement, &probe).is_none()
    {
        return MovementOutcome::MovedWrongly(rotation.unwrap_or(player.rotation()));
    }
    let fall_flying = player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING);
    let accepted = player_move_is_accepted(player, &probe);
    // `Entity.move` rewrites the flag before the packet is judged either way
    // (`Entity.java:757-761`).
    player.movement.vertical_collision_below = probe.next_vertical_collision_below;
    if !accepted {
        return MovementOutcome::MovedWrongly(rotation.unwrap_or(player.rotation()));
    }
    if movement.position.is_some() {
        player.set_position(target);
    }
    // `:1148-1156`, over the packet's own vertical delta from `lastGood`.
    player.movement.client_is_floating =
        probe.client_is_floating(player, target.y - player.last_good.y, fall_flying);
    // `:1146`: an accepted move snaps position and rotation together, which is
    // why a refused one leaves the packet's rotation unapplied.
    if let Some(rotation) = rotation {
        player.set_rotation(rotation);
    }
    player.set_on_ground(movement.on_ground);
    player
        .body_mut()
        .set_horizontal_collision(movement.horizontal_collision);
    // `handleMovePlayer` (:1157-1161): the accepted position minus the
    // one the packet started from, through `handlePlayerKnownMovement`
    // (`:2179-2185`).
    player.known_movement = Vec3d {
        x: player.position().x - start.x,
        y: player.position().y - start.y,
        z: player.position().z - start.z,
    };
    // `:1176-1178`: the accepted position is the next packet's reference.
    player.last_good = player.position();
    player.received_movement_this_tick = true;
    MovementOutcome::Accepted
}

pub(super) fn player_move_is_accepted(player: &PlayerState, probe: &PlayerMoveProbe) -> bool {
    player.game_type == GameType::Spectator
        || ((!player_moved_wrongly(player, probe.moved_distance) || !probe.old_position_clear)
            && !probe.collides_with_new_geometry)
}

pub(super) fn player_moved_wrongly(player: &PlayerState, distance: f64) -> bool {
    !player.is_changing_dimension
        && distance > MOVED_WRONGLY_DISTANCE
        && player.game_type != GameType::Creative
        && player.game_type != GameType::Spectator
        && !player.cold.fall_impulse.is_in_grace()
}

pub(super) fn apply_player_move_prefix(
    player: &mut PlayerState,
    scoreboard: &mut super::server_scoreboard::ServerScoreboard,
    movement: bbb_protocol::packets::MovePlayer,
    probe: &PlayerMoveProbe,
) -> Option<()> {
    apply_player_move_position_prefix(player, scoreboard, movement, probe)?;
    apply_player_move_velocity_prefix(player, probe, scoreboard.content_registry())
}

pub(super) fn apply_player_move_position_prefix(
    player: &mut PlayerState,
    scoreboard: &mut super::server_scoreboard::ServerScoreboard,
    movement: bbb_protocol::packets::MovePlayer,
    probe: &PlayerMoveProbe,
) -> Option<()> {
    let start = probe.packet_start.unwrap_or(player.position());
    let target = movement.position.map_or(start, clamped_target);
    // ServerGamePacketListenerImpl.java:1112-1117: the jump precedes collision refusal.
    if player.cold.last_vehicle.is_none()
        && player.on_ground()
        && !movement.on_ground
        && target.y - player.last_good.y > 0.0
    {
        let factor = probe.jump_factor?;
        jump_from_ground(player, scoreboard, factor);
    }
    apply_player_move_body_prefix(player, probe)
}

pub(super) fn apply_player_move_body_prefix(
    player: &mut PlayerState,
    probe: &PlayerMoveProbe,
) -> Option<()> {
    // Entity.java:734-761: record and write the solved body before support and velocity tails.
    if let Some(segment) = probe.contact_movement {
        crate::entity::record_body_movement(&mut player.block_contact.pending, segment);
    }
    if probe.used_stuck_multiplier {
        player.block_contact.stuck_multiplier = Vec3d::default();
        player.set_delta_movement(Vec3d::default());
    }
    if let Some(position) = probe.resolved_position {
        player.set_position(position);
    }
    player.movement.vertical_collision_below = probe.next_vertical_collision_below;
    if probe.updates_ground {
        player.set_on_ground(probe.next_vertical_collision_below);
    }
    Some(())
}

pub(super) fn apply_player_move_velocity_prefix(
    player: &mut PlayerState,
    probe: &PlayerMoveProbe,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<()> {
    // Entity.java:778-788: collision velocity changes precede the packet's verdict.
    if let Some(response) = probe.collision_response {
        player.block_contact.supporting = response.supporting;
        player.block_contact.on_ground_no_blocks = response.on_ground_no_blocks;
        let mut velocity = player.delta_movement();
        if response.clip_x {
            velocity.x = 0.0;
        }
        if response.clip_z {
            velocity.z = 0.0;
        }
        if response.vertical_collision {
            let updated = response.fall_state.and_then(|state| {
                crate::block_behavior::ground_contact::movement_after_fall_on_with_registry(
                    state,
                    true,
                    player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
                    velocity,
                    content_registry,
                )
            })?;
            velocity = updated;
        }
        player.set_delta_movement(velocity);
    }
    Some(())
}

/// `Vec3.lengthSqr` (`Vec3.java:184-186`) over two positions.
fn squared_distance(a: Vec3d, b: Vec3d) -> f64 {
    let x = a.x - b.x;
    let y = a.y - b.y;
    let z = a.z - b.z;
    x * x + y * y + z * z
}

/// ServerPlayer.java:1532-1540.
fn jump_from_ground(
    player: &mut PlayerState,
    scoreboard: &mut super::server_scoreboard::ServerScoreboard,
    block_factor: f32,
) {
    use bbb_server_types::food::{
        cause_food_exhaustion, EXHAUSTION_JUMP, EXHAUSTION_JUMP_SPRINTING,
    };

    let strength = player
        .attributes
        .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)
        .expect("player declares jump strength");
    if let Some(movement) = crate::entity::living::body_ground_jump_velocity(
        player.delta_movement(),
        strength,
        block_factor,
        player
            .active_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
            .map(|effect| effect.amplifier()),
        player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING),
        player.rotation().y_rot,
    ) {
        player.set_delta_movement(movement);
        player.needs_sync = true;
    }
    player.award_stat_once(scoreboard, custom_stat("minecraft:jump"));
    let exhaustion = if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING) {
        EXHAUSTION_JUMP_SPRINTING
    } else {
        EXHAUSTION_JUMP
    };
    cause_food_exhaustion(&mut player.food, player.abilities.invulnerable, exhaustion);
}

#[cfg(test)]
impl super::ServerWorld {
    pub(crate) fn player_stat_for_test(
        &self,
        connection_id: ConnectionId,
        stat: bbb_server_types::stats::StatKey,
    ) -> i32 {
        self.levels[connection_id].cold.stats.value(stat)
    }
}

/// `Math.round((float)Math.sqrt(..) * 100.0F)` (`ServerPlayer.java:1445`), the
/// centimetre conversion the chain below shares. The horizontal branches pass
/// no `dy`.
fn movement_centimeters(dx: f64, dy: f64, dz: f64) -> i32 {
    (((dx * dx + dy * dy + dz * dz).sqrt() as f32) * 100.0).round() as i32
}

/// `ServerPlayer.checkMovementStatistics` (ServerPlayer.java:1420-1468).
pub(super) fn check_movement_statistics(
    player: &mut PlayerState,
    scoreboard: &mut super::server_scoreboard::ServerScoreboard,
    on_climbable: bool,
) {
    use bbb_server_types::food::cause_food_exhaustion;
    // `:1421` through `didNotMove` (`:1492-1494`), and the passenger a
    // `MovePlayer` never reaches this far for anyway (`:1073-1075`).
    let delta = player.known_movement;
    if player.cold.last_vehicle.is_some() || (delta.x == 0.0 && delta.y == 0.0 && delta.z == 0.0) {
        return;
    }
    let water_stat = if player
        .shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING)
    {
        Some(("minecraft:swim_one_cm", true))
    } else if player
        .fluid_interaction
        .is_eye_in_fluid(crate::entity::FluidTag::Water)
    {
        Some(("minecraft:walk_under_water_one_cm", true))
    } else if player
        .fluid_interaction
        .is_in_fluid(crate::entity::FluidTag::Water)
    {
        Some(("minecraft:walk_on_water_one_cm", false))
    } else {
        None
    };
    if let Some((stat, vertical)) = water_stat {
        let cm = movement_centimeters(delta.x, if vertical { delta.y } else { 0.0 }, delta.z);
        if cm > 0 {
            player.award_stat(scoreboard, custom_stat(stat), cm);
            cause_food_exhaustion(
                &mut player.food,
                player.abilities.invulnerable,
                0.01_f32 * cm as f32 * 0.01_f32,
            );
        }
    } else if on_climbable {
        // `:1440-1443`, the one arm that measures a signed height rather than a
        // distance: it counts only a rise, and rounds `dy * 100.0` as a double
        // rather than through the `(float)` cast the others share.
        if delta.y > 0.0 {
            player.award_stat(
                scoreboard,
                custom_stat("minecraft:climb_one_cm"),
                (delta.y * 100.0).round() as i32,
            );
        }
    } else if player.on_ground() {
        // `:1444-1457`.
        let horizontal = movement_centimeters(delta.x, 0.0, delta.z);
        if horizontal > 0 {
            if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING) {
                player.award_stat(
                    scoreboard,
                    custom_stat("minecraft:sprint_one_cm"),
                    horizontal,
                );
                cause_food_exhaustion(
                    &mut player.food,
                    player.abilities.invulnerable,
                    0.1_f32 * horizontal as f32 * 0.01_f32,
                );
            } else if player.pose_id == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID
            {
                // `Entity.isCrouching` (`Entity.java:2631-2633`).
                player.award_stat(
                    scoreboard,
                    custom_stat("minecraft:crouch_one_cm"),
                    horizontal,
                );
            } else {
                player.award_stat(scoreboard, custom_stat("minecraft:walk_one_cm"), horizontal);
            }
        }
    } else if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING) {
        // `:1458-1460`, which awards its distance without a floor under it.
        player.award_stat(
            scoreboard,
            custom_stat("minecraft:aviate_one_cm"),
            movement_centimeters(delta.x, delta.y, delta.z),
        );
    } else {
        // `:1461-1465`.
        let horizontal = movement_centimeters(delta.x, 0.0, delta.z);
        if horizontal > 25 {
            player.award_stat(scoreboard, custom_stat("minecraft:fly_one_cm"), horizontal);
        }
    }
}

impl ServerWorld {
    // Player.java:2001-2003; LivingEntity.java:1688-1714.
    pub(super) fn player_on_climbable(&mut self, connection_id: ConnectionId) -> bool {
        self.player_on_climbable_checked(connection_id)
            .unwrap_or(false)
    }

    pub(super) fn player_on_climbable_checked(
        &mut self,
        connection_id: ConnectionId,
    ) -> Option<bool> {
        let player = self.levels.player(connection_id)?;
        if player.abilities.flying || player.game_type == GameType::Spectator {
            return Some(false);
        }
        let dimension = player.dimension;
        let pos = block_position(player.position());
        let fall_flying = player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING);
        let tags = self.level_by_id(&dimension).chunks.registry_tags_shared();
        crate::entity::living::on_climbable_in_blocks(
            pos,
            fall_flying,
            |pos| self.block_state_by_key(&dimension, pos),
            |state, tag| {
                let Some(block_id) = vanilla_block_state_block_type_id(state) else {
                    return false;
                };
                tags.as_ref()
                    .is_some_and(|tags| tags.contains_block(tag, block_id))
            },
        )
    }
}

/// `Mth.wrapDegrees(float)` (Mth.java:215-226).
pub(crate) fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

impl ServerWorld {
    /// `PlayerList.tick` (PlayerList.java:469-472): every player's latency to
    /// everyone once `sendAllPlayerInfoIn` passes 600.
    pub(super) fn tick_player_latency_broadcast(&self, events: &mut Vec<WorldEvent>) {
        if !self.tick.is_multiple_of(601) {
            return;
        }
        let entries = self
            .levels
            .player_entries()
            .map(|(_, player)| initializing_player_info_entry(player))
            .collect();
        self.broadcast(
            ClientboundPlayPacket::PlayerInfoUpdate(PlayerInfoUpdate {
                actions: vec![PlayerInfoAction::UpdateLatency],
                entries,
            }),
            events,
        );
    }
}

impl ServerWorld {
    // ServerPlayer.java:1799-1825,1869-1874: filter at the semantic send site,
    // before a later ClientInformation in the same tick can replace the options.
    pub(super) fn send_system_message(
        &self,
        connection_id: ConnectionId,
        message: bbb_protocol::packets::SystemChat,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(player) = self.levels.player(connection_id) {
            deliver_player_message(
                player.connection_id,
                player.cold.client_information.chat_visibility,
                message,
                false,
                events,
            );
        }
    }

    pub(super) fn send_command_feedback_message(
        &mut self,
        connection_id: ConnectionId,
        feedback: &crate::commands::CommandFeedback,
        visibility: bbb_protocol::packets::ClientChatVisibility,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(chat) = &feedback.chat {
            self.send_chat(connection_id, chat, events);
            return;
        }
        let message =
            bbb_protocol::packets::SystemChat::from_component(feedback.message.clone(), false);
        deliver_player_message(
            connection_id,
            visibility,
            message,
            feedback.kind == crate::commands::CommandFeedbackKind::Chat,
            events,
        );
    }

    pub(super) fn broadcast_command_message(
        &mut self,
        message: crate::commands::CommandBroadcast,
        events: &mut Vec<WorldEvent>,
    ) {
        match message {
            crate::commands::CommandBroadcast::Chat(message) => {
                self.broadcast_chat(&message, events);
            }
            crate::commands::CommandBroadcast::System(message) => {
                self.broadcast(
                    ClientboundPlayPacket::SystemChat(
                        bbb_protocol::packets::SystemChat::from_component(message, false),
                    ),
                    events,
                );
            }
            crate::commands::CommandBroadcast::Whisper { player, message } => {
                if let Some(connection_id) = self.command_player_connection(&player) {
                    self.send_chat(connection_id, &message, events);
                }
            }
        }
    }
}

pub(super) fn accepts_player_message(
    visibility: bbb_protocol::packets::ClientChatVisibility,
    overlay: bool,
    is_chat: bool,
) -> bool {
    use bbb_protocol::packets::ClientChatVisibility;
    if is_chat {
        visibility == ClientChatVisibility::Full
    } else {
        visibility != ClientChatVisibility::Hidden || overlay
    }
}

fn deliver_player_message(
    connection_id: ConnectionId,
    visibility: bbb_protocol::packets::ClientChatVisibility,
    message: bbb_protocol::packets::SystemChat,
    is_chat: bool,
    events: &mut Vec<WorldEvent>,
) {
    if accepts_player_message(visibility, message.overlay, is_chat) {
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SystemChat(message),
        ));
    }
}

#[cfg(test)]
mod climbable_stats_tests;

impl PlayerState {
    /// `ServerPlayer.getLastClientMoveIntent` (`ServerPlayer.java:2213-2217`) through
    /// `Entity.getInputVector` (`Entity.java:1657-1667`) at speed 1.
    pub(crate) fn last_client_move_intent(&self) -> Vec3d {
        let input = self.last_client_input;
        let axis = |positive: bool, negative: bool| match (positive, negative) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        };
        let left = axis(input.left, input.right);
        let forward = axis(input.forward, input.backward);
        let length_sqr: f64 = left * left + forward * forward;
        if length_sqr < 1.0e-7 {
            return Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            };
        }
        let scale = if length_sqr > 1.0 {
            length_sqr.sqrt().recip()
        } else {
            1.0
        };
        let (left, forward) = (left * scale, forward * scale);
        let radians = self.rotation().y_rot.to_radians();
        let (sin, cos) = (f64::from(radians.sin()), f64::from(radians.cos()));
        Vec3d {
            x: left * cos - forward * sin,
            y: 0.0,
            z: forward * cos + left * sin,
        }
    }
}

#[cfg(test)]
mod movement_precheck_tests {
    use super::*;
    use crate::tests::{creative_fixture_config, join_loaded, mailbox};
    use crate::world::WorldCommand;
    use bbb_protocol::packets::{MovePlayer, PlayServerbound};

    #[test]
    fn movement_precheck_handles_packet_counter_wraparound_flood_clamp_and_non_finite_velocity() {
        let (sender, receiver) = mailbox();
        let mut world = crate::world::ServerWorld::new(creative_fixture_config()).unwrap();
        world.set_test_respawn_radius(0);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

        let facts = MovementValidationFacts {
            movement_check: true,
            elytra_movement_check: true,
            runs_normally: true,
            is_passenger: false,
        };

        // Tick 0 resets first_good/last_good from the current position.
        {
            let player = world.levels.player_mut(ConnectionId(1)).unwrap();
            let pos = player.position();
            player.tick_count = 0;
            player.first_good = Vec3d {
                x: 999.0,
                y: 999.0,
                z: 999.0,
            };
            player.last_good = player.first_good;
            let outcome = movement_precheck(
                player,
                &MovePlayer {
                    position: Some(pos),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                },
                facts,
            );
            assert_eq!(outcome, None);
            assert_eq!(player.first_good, pos);
            assert_eq!(player.last_good, pos);
        }

        // Counter wraparound across i32::MAX -> i32::MIN does not panic and
        // computes delta_packets == 1.
        {
            let player = world.levels.player_mut(ConnectionId(1)).unwrap();
            let pos = player.position();
            player.tick_count = 10;
            player.received_move_packets = i32::MAX;
            player.known_move_packets = i32::MAX;
            let outcome = movement_precheck(
                player,
                &MovePlayer {
                    position: Some(Vec3d {
                        x: pos.x + 15.0,
                        y: pos.y,
                        z: pos.z,
                    }),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                },
                facts,
            );
            assert_eq!(outcome, Some(MovementOutcome::TooQuickly));
            assert_eq!(player.received_move_packets, i32::MIN);
        }

        // Flooding > 5 move packets in one tick clamps delta_packets to 1 rather
        // than expanding the distance budget.
        {
            let player = world.levels.player_mut(ConnectionId(1)).unwrap();
            let pos = player.position();
            player.known_move_packets = 0;
            player.received_move_packets = 5;
            // 15 blocks squared = 225 > 100 * 1 (would be allowed if delta_packets were 6).
            let outcome = movement_precheck(
                player,
                &MovePlayer {
                    position: Some(Vec3d {
                        x: pos.x + 15.0,
                        y: pos.y,
                        z: pos.z,
                    }),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                },
                facts,
            );
            assert_eq!(outcome, Some(MovementOutcome::TooQuickly));
        }

        // Non-finite delta_movement cannot bypass the TooQuickly speed check.
        {
            let player = world.levels.player_mut(ConnectionId(1)).unwrap();
            let pos = player.position();
            player.known_move_packets = 10;
            player.received_move_packets = 10;
            player.set_delta_movement(Vec3d {
                x: f64::NAN,
                y: 0.0,
                z: 0.0,
            });
            let outcome = movement_precheck(
                player,
                &MovePlayer {
                    position: Some(Vec3d {
                        x: pos.x + 25.0,
                        y: pos.y,
                        z: pos.z,
                    }),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                },
                facts,
            );
            assert_eq!(outcome, Some(MovementOutcome::TooQuickly));
            player.set_delta_movement(Vec3d::default());
        }

        // Ordinary valid step still succeeds through the full play pipeline.
        let pos = world.publication().players[0].position;
        let mut events = Vec::new();
        world.apply_command(
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::MovePlayer(MovePlayer {
                    position: Some(Vec3d {
                        x: pos.x + 0.25,
                        y: pos.y,
                        z: pos.z,
                    }),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                }),
            },
            &mut events,
        );
        assert!(events.iter().all(|event| !matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )));
    }
}
