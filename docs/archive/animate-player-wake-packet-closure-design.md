# Animate player-wake packet coverage closure

## Goal and completion contract

Close the remaining canonical-apply and runtime-side-effect coverage cells for
`ClientboundAnimatePacket`. Action `2` must run the Vanilla 26.1 player wake
transition only for a `Player`: clear a loaded bed's occupied state, choose the
Vanilla safe stand-up position or fallback, update position and look direction,
set the standing pose, clear `sleepingPos`, and set the player's client-owned
`sleepCounter` to `100`. The local player mirror must move in the same packet
application, and the next native frame must render Vanilla's full-screen
sleep-fade overlay from that canonical counter.

Completion means every player entity owns a spawn-fresh, snapshot-safe sleep
counter; scheduled client ticks reproduce the sleeping `0..100` hold and awake
`100..109 -> 0` fade lifecycle; action `2` is harmless for non-player and absent
entities; loaded-bed, missing/unloaded-bed, remote-player, and local-player
branches are regression-tested; actions `0`, `3`, `4`, and `5` retain their
existing swing and tracking-emitter behavior; the sleep overlay has the exact
Vanilla color, alpha curve, visibility, and HUD ordering; all five packet
coverage dimensions are `complete`; the milestone is indexed; and the full
gate plus the applicable playable smoke pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` keeps the existing exact VarInt entity id plus unsigned action
  byte decoder. There is no wire change in this slice.
- `bbb-world` owns action dispatch, the `Player` type gate, bed mutation and
  safe-dismount search, player sleep counters, entity transform/metadata
  mutation, scheduled counter ticks, and the local-player pose mirror.
- A player-only ECS component owns `sleepCounter`; projected `EntityState`
  snapshots retain it only for player entities. Re-add, replacement, level
  generation, and snapshot publication therefore cannot leak the counter
  across entity incarnations or sessions.
- `bbb-native` reads only the canonical local player entity counter and derives
  the frame overlay. It does not retain a second timer.
- `bbb-renderer` owns one fixed-color full-surface alpha-blended fill. It is
  submitted after boss bars and before overlay message/title/chat/player-list
  content, and remains visible when ordinary HUD elements are hidden.
- The existing bed safe-dismount geometry and danger checks remain the single
  collision implementation for both living-death wake-up and player action
  `2`; this slice may refactor their shared transition builder without changing
  its search order.

This slice does not implement sleeping initiation, bed-use validation, the
sleep screen or its leave-bed command, server-side sleeping-player lists, night
skip, respawn positions, cat morning gifts, or general collision work. It does
not add in-game configuration UI or change the other animate action ids.

## Vanilla and integration contract

`ClientPacketListener.handleAnimate` resolves the entity and, for action `2`,
casts it to `Player` and calls `stopSleepInBed(false, false)`.
`Player.stopSleepInBed` calls `LivingEntity.stopSleeping` and then assigns
`sleepCounter = 100`. bbb safely gates malformed non-player action-2 packets
instead of reproducing the Java cast failure, while preserving the existing
packet accounting and `lastAnimationAction` observation for every known
entity.

`LivingEntity.stopSleeping` examines `sleepingPos` only when its chunk is
loaded. If the block is a bed, it clears `OCCUPIED`, calls
`BedBlock.findStandUpPosition`, falls back to the bed block center at
`y + 1.1`, faces back toward the bed center, moves there, and sets `xRot = 0`.
It then unconditionally sets `Pose.STANDING`, reapplies the resulting position,
and clears `sleepingPos`. Thus a player with no valid loaded bed retains its
transform but still becomes standing, clears the sleeping position, and gets
counter `100`. The local-player pose mirror updates atomically with the entity:
valid-bed wake-up replaces position/yaw/xrot; every wake-up clears the local
sneaking/swimming pose flags; unrelated movement facts and velocity are
preserved. The old-pose mirror is reset to the same result so the wake teleport
does not introduce a renderer-only interpolation path.

`Player.tick` increments the counter while sleeping and clamps it to `100`.
Once no longer sleeping, a positive counter increments and resets to `0` when
it reaches `110`. Bulk bbb client ticks must produce the same result without
iterating once per requested tick. `Gui.extractSleepOverlay` draws whenever the
local counter is positive. For `counter <= 100`, `amount = counter / 100`; for
`counter > 100`, `amount = 1 - (counter - 100) / 10`. Its ARGB color is
`(int)(220 * amount) << 24 | 0x101020`; no partial-tick interpolation is used.
The overlay extraction sits outside `Options.hideGui`, calls `nextStratum`, and
is ordered after camera/hotbar/effect/boss content but before demo,
scoreboard, overlay message, title, chat, player list, and subtitles.

Authoritative local sources are
`net/minecraft/client/multiplayer/ClientPacketListener.java#handleAnimate`,
`net/minecraft/world/entity/LivingEntity.java#stopSleeping`,
`net/minecraft/world/entity/player/Player.java#tick`,
`net/minecraft/world/entity/player/Player.java#stopSleepInBed`,
`net/minecraft/world/level/block/BedBlock.java#findStandUpPosition`, and
`net/minecraft/client/gui/Gui.java#extractRenderState` /
`#extractSleepOverlay` under the Vanilla 26.1 source tree.

## Serial stages and verification

1. Add the player-only sleep-counter component and snapshot field; share the
   existing bed transition builder; apply action `2` with exact player,
   transform, metadata, local-pose, and counter behavior; and advance counters
   in scheduled client ticks. Focused tests cover loaded/fallback/missing bed,
   non-player/absent entity, remote/local ownership, spawn/re-add/snapshot
   reset, and the sleeping/awake counter boundaries with
   `scripts/cargo-dev.sh test -p bbb-world player_wake` plus the existing
   animation and level-time regressions.
2. Add the native counter-to-overlay projection and renderer full-surface fill
   in the frozen HUD order. Focused tests assert every alpha boundary,
   `0x101020`, F1 independence, command ordering, and an ordered network
   event-to-world-to-renderer regression with
   `scripts/cargo-dev.sh test -p bbb-renderer sleep_overlay` and
   `scripts/cargo-dev.sh test -p bbb-native animate_player_wake`.
3. Mark the packet row complete, replace its reverse partial guard, regenerate
   and check the ledger, remove only the exact unsupported-feature bullet
   closed by this slice, archive this contract, and append the milestone
   index. Run `scripts/cargo-dev.sh test -p bbb-protocol packet_coverage`,
   review the integrated diff, then run `scripts/cargo-dev.sh gate`,
   `scripts/playable-gate.sh live`, and `scripts/playable-gate.sh auto`. A
   missing configured Java executable is recorded as live-environment
   unavailable rather than a product failure; the checked-in auto fixture must
   pass.

The fully gated `e29e5cd35` is the pre-start checkpoint. Each stage is a clean,
buildable commit. World failures stop before renderer work; native/renderer
failures stop before coverage closeout. The final closeout is the fourth commit
including this contract and therefore forces a full gate. A failed full gate
is localized from the pre-start checkpoint only when focused tests do not
identify it. Failures are fixed forward without rewriting history.

Closeout completed all five packet-coverage cells after canonical player wake,
sleep-counter lifecycle, ordered network-to-HUD projection, and renderer
overlay regressions passed.
