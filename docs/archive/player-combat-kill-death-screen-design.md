# Player Combat Kill and Death Screen Closure

Status: closed and archived after the packet coverage row reached complete in every dimension.

This document freezes the engineering contract for closing Vanilla 26.1
`ClientboundPlayerCombatKillPacket` from the full-gated baseline
`e24e73c10`.

## Goal and Completion Standard

When the packet names the current local player, the client must either open the
Vanilla death screen or immediately send `ServerboundClientCommandPacket`
`PERFORM_RESPAWN`, according to the current `showDeathScreen` flag. A packet for
any other entity is an observed no-op.

The screen closure includes:

- exact rich cause-of-death Component retention, including styled text and
  trusted link actions;
- the synchronized player score, login `hardcore` flag, ordinary versus
  Hardcore title/button labels, the 20-screen-tick button delay, and the
  non-pausing/non-Escape-close lifecycle;
- Respawn/Spectate, Title Screen, and ordinary-mode quit confirmation actions,
  with each respawn or disconnect command emitted at most once for the active
  generation;
- the Vanilla red death gradient, 2x centered title, cause/score rows, widgets,
  hover/focus, narration, and deterministic renderer hit testing/readback; and
- generation-safe clearing on screen replacement, respawn, login,
  reconfiguration, disconnect, or level replacement.

Closure requires every coverage cell for `CLIENTBOUND_PLAYER_COMBAT_KILL` to be
`complete`, removal of its TODO, semantic focused tests at each ownership
boundary, a final full gate, and the recorded playable fixture smoke.

## Non-Goals

- Damage, death, score, game mode, respawn location, or Hardcore authority is
  never inferred from the kill packet.
- The client does not implement Mojang player-report draft state or its
  `icon/draft_report` badge.
- A standalone main-menu/server-list implementation is not introduced. “Title
  Screen” ends the current multiplayer connection through the existing typed
  disconnect command and clears the active screen.
- This slice does not change DeathScreen accessibility backdrop options,
  server-owned death messages, signed-chat reporting, or the unrelated Win
  screen.

## Owners and Frozen Boundaries

- `bbb-protocol::packets::player_actions` owns the VarInt player id and exact
  terminal `Component` decode. The packet carries one `RichTextComponent`; no
  second plain/styled/opaque representation is added.
- `bbb-world::client::combat` owns the observed event, local-player identity
  gate, immutable death-screen state, score/default resolution through the
  generated Player metadata accessor, and generation-safe screen lifecycle.
  It returns a narrow apply outcome for the runtime side effect.
- `bbb-world::PlayApplyEffects` owns automatic `PERFORM_RESPAWN` delivery.
  Native queues the existing `NetCommand::PerformRespawn`; state-only callers
  retain the identical canonical decision without inventing a network owner.
- `bbb-native::input::death_screen` owns focus, the 20-tick enable delay,
  confirmation state, one-shot input commands, and platform link requests.
  World-to-render data enters only through `RendererFrame`.
- `bbb-renderer::hud::death_screen` owns sanitization, exact layout, draw
  commands, rich-text/button hit testing, and offscreen readback. It receives
  value state only and does not inspect the world or connection.

## Vanilla 26.1 Evidence

The authoritative sources are:

- `ClientPacketListener.handlePlayerCombatKill`: resolve `level.getEntity`,
  compare object identity with `minecraft.player`, then choose
  `DeathScreen` versus `LocalPlayer.respawn`;
- `ClientboundPlayerCombatKillPacket.STREAM_CODEC`: VarInt player id followed
  by a terminal trusted `Component`;
- `DeathScreen`: title key by Hardcore mode, yellow score argument, 200x20
  buttons at `height / 4 + 72/+96`, disabled for 20 screen ticks, red gradient,
  2x title at y=30, cause at y=85, score at y=100, URL click handling, no pause,
  no Escape close, and portal allowance;
- `Player.DATA_SCORE_ID` / `Player.getScore`: generated INT metadata with
  define-time default zero; and
- `LocalPlayer.respawn` plus
  `ServerboundClientCommandPacket.Action.PERFORM_RESPAWN`.

The main agent rechecks these local sources under
`/home/zgy/Work/mc-code/sources/26.1` before changing each boundary.

## Ordered Stages and Focused Tests

### A. Rich packet and canonical decision

Replace the lossy message string with `RichTextComponent`. Resolve the generated
Player score accessor, store a generation-tagged death screen only for the
current local player, and return automatic-respawn only when
`showDeathScreen == false`. Screen replacement and level lifecycle clear it.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-protocol player_combat_kill
scripts/cargo-dev.sh test -p bbb-world player_combat_kill
scripts/cargo-dev.sh test -p bbb-world player_score
scripts/cargo-dev.sh test -p bbb-native player_combat_kill
```

### B. Native input, commands, and narration

Synchronize one input owner from the canonical generation. Tick the enable delay
on the client tick, handle focus/mouse/keyboard without Escape dismissal, queue
Respawn or Disconnect once, preserve ordinary-mode quit confirmation, and
revalidate rich link actions against the active generation.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-native death_screen
scripts/cargo-dev.sh test -p bbb-native perform_respawn
scripts/cargo-dev.sh test -p bbb-net perform_respawn
```

### C. Renderer presentation

Add a dedicated HUD value/module. Draw the ordinary HUD first, then the
DeathScreen gradient and foreground without menu blur. Share the exact layout
between draw and hit testing; add semantic draw assertions and deterministic
offscreen pixels for the background, title, cause/score, disabled/enabled
buttons, hover, and confirmation.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-renderer hud_death_screen
scripts/cargo-dev.sh test -p bbb-native hud_death_screen
scripts/cargo-dev.sh test -p bbb-renderer offscreen_death_screen
```

### D. Coverage and closeout

Update the sparse packet override, regenerate the packet ledger, remove the
closed TODO, archive this contract, and add the milestone index.

Run:

```text
scripts/vanilla-source-tools.sh python -m unittest scripts.tests.test_packet_coverage_ledger
scripts/cargo-dev.sh gate
scripts/playable-gate.sh fixture
scripts/audit-p0.sh
```

## Checkpoints, Failures, and Safe Stops

The contract, rich canonical decision, native input/runtime, renderer surface,
and coverage closeout are separate clean commits. Run the full gate after at
most four commits and again at final HEAD. Fix focused or gate failures forward;
use `batch-gate` only if the introducing commit is unclear.

Safe stops are a clean contract commit, a rich canonical outcome with both
native and state-only sinks green, a generation-safe native input state with no
renderer dependency, or a complete renderer value surface before ledger
closeout. No new world state, network request, or renderer surface lands without
its consumer and focused proof in the same checkpoint.
