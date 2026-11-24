//! The idle kick `ServerGamePacketListenerImpl.tick` performs
//! (`ServerGamePacketListenerImpl.java:296-306`) and the last-action time it
//! reads.
//!
//! The clock is `Util.getMillis()` (`Util.java:137-139`), which is
//! `System::nanoTime` divided down (`Util.java:106`) rather than epoch time,
//! so the count here is process-monotonic too and shares its origin with the
//! profiler clock `/debug` reports.

use crate::connection::ConnectionId;
use bbb_protocol::packets::{PlayServerbound, Vec3d};

use super::{ServerWorld, WorldCloseReason, WorldEvent};

/// `Util.getMillis()` (`Util.java:137-139`).
#[cfg(not(test))]
pub(crate) fn monotonic_millis() -> i64 {
    super::command_execution::debug_now_nanos() / 1_000_000
}

/// The same reading, from a clock the tests own.
///
/// The shortest window `/setidletimeout` can arm is a minute, which is not a
/// thing a test can spend, and winding the sessions back instead would put
/// `lastActionTime` where the world cannot -- below zero, which is the one
/// value the guard below is about. A clock the test moves keeps every reading
/// a value the running server could have produced.
#[cfg(test)]
pub(crate) fn monotonic_millis() -> i64 {
    TEST_CLOCK_MILLIS.with(std::cell::Cell::get)
}

#[cfg(test)]
thread_local! {
    /// Past zero at rest, the way a platform clock the server did not start is.
    static TEST_CLOCK_MILLIS: std::cell::Cell<i64> = const { std::cell::Cell::new(1) };
}

#[cfg(test)]
pub(crate) fn advance_clock_for_test(millis: i64) {
    TEST_CLOCK_MILLIS.with(|clock| clock.set(clock.get() + millis));
}

#[cfg(test)]
pub(crate) fn set_clock_for_test(millis: i64) {
    TEST_CLOCK_MILLIS.with(|clock| clock.set(millis));
}

/// `handlePlayerKnownMovement`'s threshold (`ServerGamePacketListenerImpl.java:2180`),
/// whose `1.0E-5F` literal is a float widened for the comparison.
const KNOWN_MOVEMENT_EPSILON: f64 = 1.0e-5_f32 as f64;

/// Under what condition one inbound packet counts as activity.
///
/// Vanilla spends seventeen separate `resetLastActionTime` calls on this, each
/// inside the handler that already knows its own guard. Naming the guard per
/// packet keeps the ones that matter -- a client that is merely connected
/// still sends movement, keep-alives and acknowledgements every tick, and the
/// packets left out here are exactly what makes the kick reachable at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PlayerActivity {
    /// Resets with no guard beyond the handler being reached.
    Always,
    /// The handlers whose reset sits inside `if (this.hasClientLoaded())`.
    WhenLoaded,
    /// The two movement handlers, which reset through
    /// `handlePlayerKnownMovement` (`:501`, `:1161`) and so only when the
    /// player actually moved.
    IfMoved,
    Never,
}

/// Which of those guards each packet's handler carries.
pub(super) fn packet_activity(packet: &PlayServerbound) -> PlayerActivity {
    match packet {
        // `handleSetCarriedItem` (`:1470-1478`), `handleAnimate`
        // (`:1682-1684`), `handleClientCommand` (`:1859-1861`),
        // `handleContainerClick` (`:1915-1917`), `handlePlaceRecipe`
        // (`:1954-1956`), `handleContainerButtonClick` (`:1985-1987`) and
        // `updateSignText` (`:2033-2034`).
        PlayServerbound::SetCarriedItem { .. }
        | PlayServerbound::Swing { .. }
        | PlayServerbound::ClientCommand(_)
        | PlayServerbound::ContainerClick(_)
        | PlayServerbound::PlaceRecipe(_)
        | PlayServerbound::ContainerButtonClick(_)
        | PlayServerbound::SignUpdate(_)
        // `tryHandleChat` (`:1613-1620`), which all three chat entries reach
        // (`:1488`, `:1509`, `:1529`).
        | PlayServerbound::Chat(_)
        | PlayServerbound::ChatCommand { .. }
        | PlayServerbound::ChatCommandSigned(_) => PlayerActivity::Always,
        // `handlePlayerInput` (`:417-421`), `handlePlayerAction`
        // (`:1258-1262`), `handleUseItemOn` (`:1328-1344`), `handleUseItem`
        // (`:1398-1405`), `handlePlayerCommand` (`:1689-1692`),
        // `handleAttack` (`:1784-1789`) and `handleInteract` (`:1813-1818`).
        // SpectateEntity resets inside its combined loaded/spectator gate.
        PlayServerbound::PlayerInput(_)
        | PlayServerbound::PlayerAction(_)
        | PlayServerbound::UseItemOn(_)
        | PlayServerbound::UseItem(_)
        | PlayServerbound::PlayerCommand(_)
        | PlayServerbound::Attack(_)
        | PlayServerbound::Interact(_) => PlayerActivity::WhenLoaded,
        PlayServerbound::MovePlayer(_) | PlayServerbound::MoveVehicle(_) => {
            PlayerActivity::IfMoved
        }
        _ => PlayerActivity::Never,
    }
}

impl ServerWorld {
    /// `resetLastActionTime` (`ServerPlayer.java:1890-1892`) for the packet
    /// that has just run, over the position the session held before it.
    ///
    /// The movement arm measures the player rather than the vehicle vanilla
    /// hands `handlePlayerKnownMovement` at `:500`; a boat move that is
    /// accepted snaps its passenger onto the new placement, so the two deltas
    /// differ only by the constant riding offset.
    pub(super) fn note_player_activity(
        &mut self,
        connection_id: ConnectionId,
        activity: PlayerActivity,
        before: Option<Vec3d>,
    ) {
        if activity == PlayerActivity::Never {
            return;
        }
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        match activity {
            PlayerActivity::WhenLoaded if !player.client_loaded() => return,
            PlayerActivity::IfMoved => {
                let Some(before) = before else {
                    return;
                };
                let after = player.position();
                let moved = (after.x - before.x).powi(2)
                    + (after.y - before.y).powi(2)
                    + (after.z - before.z).powi(2);
                if moved <= KNOWN_MOVEMENT_EPSILON {
                    return;
                }
            }
            _ => {}
        }
        player.cold.last_action_millis = monotonic_millis();
    }

    /// The idle arm of `ServerGamePacketListenerImpl.tick` (`:300-305`), which
    /// sits beside `keepConnectionAlive` and so runs on every tick rather than
    /// on the keep-alive cadence.
    pub(super) fn tick_idle_timeout(&mut self, events: &mut Vec<WorldEvent>) {
        if self.cold.player_idle_timeout <= 0 {
            return;
        }
        let allowance = i64::from(self.cold.player_idle_timeout) * 60_000;
        let now = monotonic_millis();
        let idle = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                player.cold.last_action_millis > 0
                    && now - player.cold.last_action_millis > allowance
                    && !player.cold.won_game
            })
            .map(|(&connection_id, _)| connection_id)
            .collect::<Vec<_>>();
        for connection_id in idle {
            self.remove_player(connection_id, events);
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::IdleTimeout,
            });
        }
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn player_idle_timeout_for_test(&self) -> i32 {
        self.cold.player_idle_timeout
    }

    pub(crate) fn last_action_millis_for_test(&self, connection_id: ConnectionId) -> i64 {
        self.levels[connection_id].cold.last_action_millis
    }

    pub(crate) fn set_won_game_for_test(&mut self, connection_id: ConnectionId) {
        self.levels[connection_id].cold.won_game = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{InteractionHand, MovePlayer, PlayerInput};

    /// Every packet a connected client sends on its own, without the player
    /// touching anything. If any of these counted, the kick would never fire.
    #[test]
    fn the_packets_an_idle_client_sends_by_itself_are_not_activity() {
        for packet in [
            PlayServerbound::KeepAlive { id: 1 },
            PlayServerbound::AcceptTeleportation { id: 1 },
            PlayServerbound::ClientTickEnd,
            PlayServerbound::PlayerLoaded,
            PlayServerbound::ChatAck { offset: 1 },
            PlayServerbound::PingRequest { time: 1 },
            PlayServerbound::Pong { id: 1 },
            PlayServerbound::ClientInformation(Default::default()),
            PlayServerbound::ChunkBatchReceived {
                desired_chunks_per_tick: 1.0,
            },
        ] {
            assert_eq!(
                packet_activity(&packet),
                PlayerActivity::Never,
                "{packet:?}"
            );
        }
    }

    /// `handleAnimate` (`:1682-1684`) resets before it does anything else;
    /// `handlePlayerInput` (`:417-421`) resets inside `hasClientLoaded()`.
    #[test]
    fn the_two_reset_shapes_are_told_apart_by_the_guard_around_the_call() {
        assert_eq!(
            packet_activity(&PlayServerbound::Swing {
                hand: InteractionHand::MainHand,
            }),
            PlayerActivity::Always
        );
        assert_eq!(
            packet_activity(&PlayServerbound::PlayerInput(PlayerInput::default())),
            PlayerActivity::WhenLoaded
        );
        assert_eq!(
            packet_activity(&PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            })),
            PlayerActivity::IfMoved
        );
    }

    /// `1.0E-5F` is a float literal in a double comparison, so the bound is the
    /// widened float rather than the double `1e-5`.
    #[test]
    fn the_movement_threshold_is_the_widened_float_literal() {
        assert_ne!(KNOWN_MOVEMENT_EPSILON, 1.0e-5_f64);
        assert_eq!(KNOWN_MOVEMENT_EPSILON, f64::from(1.0e-5_f32));
    }
}
