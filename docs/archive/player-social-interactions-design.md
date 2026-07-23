# Player Social Manager and Social Interactions Closure

This document records the engineering contract that closed Vanilla 26.1
`PlayerSocialManager`, `SocialInteractionsScreen`, and the remaining runtime/test
cells of `CLIENTBOUND_PLAYER_INFO_REMOVE` from the fully gated baseline
`f404fb1b3`.

## Goal and Completion Standard

The native multiplayer client must expose the default P-key Social Interactions
screen and the PauseScreen Player Reporting entry. The screen must track active,
seen, recently signed-chatting, hidden, blocked, and removed players; filter by a
bounded case-insensitive search; retain removed rows while the screen is open;
and update immediately when PlayerInfo add/remove packets arrive.

The closure includes:

- one canonical player-social owner for discovered names and the existing
  chat-listener hidden/blocked sets, without a second filtering decision;
- Vanilla All/Hidden/Blocked membership, offline/recent ordering and status
  labels, hide/show mutation, active-player count, and session/reset lifecycle;
- P/Escape/Tab/arrow/Enter/mouse/search/scroll input, return-to-pause behavior,
  the fixed blocking-help link confirmation, and generation-safe platform
  requests;
- Vanilla screen/background/search/list/button geometry, dynamic or profiled
  default 24-pixel faces, removed shading, hover/focus, localization,
  tooltips, and screen narration; and
- semantic world/native/renderer tests, deterministic offscreen pixels, removal
  of the ledger TODO, and all five `CLIENTBOUND_PLAYER_INFO_REMOVE` coverage
  cells marked `complete`.

## Non-Goals

- Mojang account sign-in, `UserApiService.refreshBlockList`, abuse-report
  composition/submission, report drafts, and the `icon/draft_report` badge stay
  outside the repo-native client boundary. A startup-resolved blocked UUID set
  is the complete blocked-list input until that external Services owner exists.
- This slice does not add in-game configuration UI or persistent server-list
  storage. Hidden players follow Vanilla's process-local manager lifetime;
  server labeling uses the active command-line/Transfer target.
- PlayerTabOverlay heads, ping sprites, header/footer rich Components, and
  unrelated PauseScreen options/actions remain in their existing queues.
- Singleplayer and integrated-server availability branches are not introduced;
  `bbb` remains an external multiplayer client.

## Owners and Frozen Boundaries

- `bbb-world::client::social` owns `PlayerSocialState`, the discovered
  name-to-UUID map, active server label, and deterministic social-player query.
  Hidden/blocked mutations delegate to the one
  `ClientChatState.presentation` owner; no duplicate UUID set is stored.
- `bbb-world::ClientUiState` owns only whether the Social Interactions screen is
  open and its monotonically increasing generation. Other screens, level
  replacement, reconfiguration, and disconnect clear it through the existing
  screen replacement lifecycle.
- `bbb-world::client::player_info` records discovered AddPlayer profiles.
  PlayerInfoRemove still removes active/listed state while preserving Vanilla's
  seen/discovered information. It does not own screen widgets.
- `bbb-native::input::social` owns page, search edit state, scroll, focus,
  cursor, rows retained by an open screen, return-to-pause, link confirmation,
  and one-shot platform requests. Its synchronization observes the canonical
  screen generation plus active/seen/chat state and marks a cached active row
  removed before any rebuild can discard it.
- `bbb-native::runtime::render_extract` projects a value-only
  `HudSocialInteractionsScreen` through `RendererFrame`. The simulation effect
  stream gains the same mouse/keyboard narration scheduling shape already used
  by StatsScreen, DeathScreen, and the Win/Demo screen.
- `bbb-renderer::hud::social` owns sanitization, shared draw/hit layout,
  scrolling clip, dynamic/default face lookup, sprite/text commands, and
  offscreen readback. Native accepts only generation-tagged renderer hit
  targets and revalidates the UUID/action against its current screen state.

## Vanilla 26.1 Evidence

The authoritative local sources are:

- `ClientPacketListener.handlePlayerInfoUpdate` /
  `handlePlayerInfoRemove`: call `PlayerSocialManager.addPlayer` and
  `removePlayer` around the active PlayerInfo map lifecycle;
- `PlayerSocialManager`: process-local hidden set, Services-backed blocked
  query, discovered-name retention, and open-screen add/remove callbacks;
- `Minecraft.handleKeybinds`: consume the P binding and open
  `SocialInteractionsScreen` in multiplayer;
- `SocialInteractionsScreen`: 236-pixel background, y=45 tabs, y=74 search,
  y=88 list, 36-pixel rows, All/Hidden/Blocked membership, lower-case
  16-character search, blocking-help button, non-pausing lifecycle, and
  P-to-close when search is not focused;
- `SocialInteractionsPlayerList`: active/seen/signed-chat sources, stable
  category/name ordering, page filtering, and removed-row retention; and
- `PlayerEntry`: 24-pixel face, active/removed fills and shade, status strings,
  hide/show buttons, delayed tooltips, and narration.

The fixed help target is `CommonLinks.BLOCKING_HELP`
(`https://aka.ms/javablocking`). The main agent rechecks these sources under
`/home/zgy/Work/mc-code/sources/26.1` before changing each boundary.

## Ordered Stages and Focused Tests

### A. Canonical social manager and packet lifecycle

Add the social owner and screen generation, route AddPlayer discovery through
the authoritative player-info apply path, preserve removed/seen/discovered
identity, delegate hide/show/blocked checks to chat presentation, and cover
screen replacement plus level/session lifecycle.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world player_social
scripts/cargo-dev.sh test -p bbb-world player_info_remove
scripts/cargo-dev.sh test -p bbb-world social_screen
```

### B. Native input, retained rows, links, and narration scheduling

Add the dedicated native input module. Open from P and the pause button, preserve
the correct return screen, synchronize packet add/remove transitions, implement
page/search/scroll/focus/hide/show behavior, and route the trusted help link plus
screen narration through ordered effects.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-native social_interactions
scripts/cargo-dev.sh test -p bbb-native player_info_remove
scripts/cargo-dev.sh test -p bbb-native screen_narration
```

### C. Renderer presentation and shared hit layout

Add the dedicated HUD value/module, load the official sprites through the
existing HUD atlas owner, reuse the bounded dynamic/default player-face cache,
draw the list after the ordinary screen background, and share exact geometry
between draw and generation-tagged hit testing. Add semantic draw assertions
and deterministic offscreen pixels for tabs/search, active/removed rows,
faces, hide/show hover, scrolling, empty pages, and link confirmation.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-renderer hud_social_interactions
scripts/cargo-dev.sh test -p bbb-native hud_social_interactions
scripts/cargo-dev.sh test -p bbb-renderer offscreen_social_interactions
```

### D. Coverage and closeout

Update the sparse packet override, regenerate the packet ledger, remove the
closed TODO, archive this contract, and add the engineering milestone index.

Run:

```text
scripts/vanilla-source-tools.sh python -m unittest scripts.tests.test_packet_coverage_ledger
scripts/cargo-dev.sh gate
scripts/playable-gate.sh fixture
scripts/audit-p0.sh
```

## Checkpoints, Failures, and Safe Stops

The contract, canonical social owner, native interaction lifecycle, renderer
surface, and coverage closeout are separate clean commits. The baseline is
already fully gated; run the next full gate after the renderer milestone (four
commits including this contract) and again at final HEAD. Focused or gate
failures are fixed forward; `batch-gate` is used only when the introducing
commit cannot otherwise be identified.

Safe stops are a clean contract commit; a canonical manager with packet/reset
tests and no input dependency; a generation-safe native screen whose retained
rows, link requests, and narration schedule have focused proof; or a complete
renderer value surface before ledger closeout. No duplicate hidden/blocked
owner, untagged renderer hit, stale platform request, or screen-only packet
switch may land at a checkpoint.
