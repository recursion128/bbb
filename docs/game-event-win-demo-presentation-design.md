# Game-event WinScreen and demo presentation closure

Status: active engineering contract.

## Goal and completion

Close the remaining vanilla 26.1 presentation behavior owned by
`ClientboundGameEventPacket` types 0, 4, and 5. Completion requires:

- type 0 reaches the translated client-system chat HUD and queued system
  narration already represented by canonical chat state;
- type 4 replaces the active screen with the poem-and-credits `WinScreen`,
  plays credits music, implements vanilla scrolling controls and completion,
  and queues exactly one `ServerboundClientCommandPacket(PERFORM_RESPAWN)`;
- type 5 parameter 0 opens the demo help popup with the movement/inventory
  explanation and trusted purchase-link confirmation, while parameters
  101–104 append the exact translated client-system hint and queue the same
  text for system narration;
- the win screen loads selected-pack `texts/end.txt`, `texts/credits.json`, and
  `texts/postcredits.txt`, renders the portal background, title logo, formatted
  lines, and credits vignette with vanilla geometry and ordering, and responds
  transactionally to resource reload;
- packet coverage, the feature ledger, focused tests, full workspace gates,
  the applicable playable fixture, and the P0 audit close at the final tip.

## Authority and non-goals

Authoritative sources are vanilla 26.1 `ClientPacketListener.handleGameEvent`,
`WinScreen`, `PopupScreen`, `ConfirmLinkScreen`, `LogoRenderer`,
`ServerboundClientCommandPacket`, `Music`, `Musics`, `CommonComponents`, and
the selected resource-pack files under `assets/minecraft/texts/`.

This slice does not implement `DeathScreen` or
`ClientboundPlayerCombatKillPacket`, the demo session's remaining-time HUD, an
IntegratedServer, the non-poem credits button path, a configurable key-binding
screen, or arbitrary web navigation. It does not change the protocol wire
shape or add a second play-packet dispatcher.

## Frozen semantics and ownership

- `bbb-world::apply_play_packet` remains the only clientbound play-packet
  dispatcher. `WorldStore::apply_game_event` owns the canonical event counters,
  the active server-requested win/demo screen marker and generation, and rich
  translatable client-system chat/narration requests. Serializable world state
  stores no wall clock, localized strings, pack bytes, GPU handles, or platform
  effects.
- Opening win or demo help uses the existing `ClientUiState::replace_screen`
  policy before installing one mutually exclusive `ClientGameEventScreenState`.
  Login/configuration level clearing removes this state. A later supported
  screen replacement closes it without treating replacement as user
  completion.
- Type 0 and type 5 hints enter the existing canonical client-system chat and
  pending-narration bridge as rich translatable components. Demo hint
  parameters are exactly 101 movement, 102 jump, 103 inventory, and 104 day-six
  screenshot; all other demo parameters are recorded but add no hint.
  Projection supplies the startup fixed key labels W/A/S/D, Space, E, and F2
  because this client has no in-game key-binding configuration.
- `bbb-native::input::game_event_screen` owns transient interaction state:
  generation synchronization, focus/hover, win scroll position, direction,
  Ctrl modifier count, Space speed-up, and demo link-confirmation state.
  Production and tests call the same sanitized render-delta transition
  methods.
- A poem win screen starts at scroll position zero, direction `1`, and base
  speed `0.5` pixels per rendered delta tick. Up changes direction to `-1`
  while held; either Ctrl key contributes one modifier; Space toggles speed-up.
  Speed-up is `0.5 * (5 + 15 * ctrl_count) * direction`. Ordinary extraction
  advances by `max(0, scroll + delta_ticks * speed)`. Escape, ordinary close,
  or scrolling past `totalScrollLength + 2 * height + 24` completes once:
  close the canonical marker and queue the existing
  `NetCommand::PerformRespawn`. Packet replacement, disconnect, or
  configuration transition does not emit respawn.
- Demo help has vanilla's two buttons. Later closes the popup and requests
  cursor capture. Buy replaces it with a trusted-link confirmation for
  `https://aka.ms/BuyMinecraftJava` whose parent is null; Open emits the
  existing `OpenUri` platform effect, Copy emits the existing clipboard
  effect, and every choice closes to gameplay. No URI is opened before
  explicit confirmation.
- `bbb-pack` adds one bounded text-resource reader sharing
  `ResourceDataGenerationBudget`, then owns bounded raw poem text and credits
  JSON documents selected by pack precedence. `bbb-native` parses those
  documents into `WinScreenResources`, substitutes the startup username for
  `PLAYERNAME`, uses the fixed vanilla obfuscation seed `8124371`, and wraps
  poem lines to 256 pixels with the active font. A missing or invalid
  individual text file contributes no lines from that file and does not reject
  unrelated valid files; exhaustion of the shared generation budget rejects
  the candidate reload.
- Credits line construction follows vanilla order and style: separators are
  centered white and not narrated; sections and nonempty disciplines are
  centered yellow and narrated; titles are left gray and narrated; names have
  eleven leading spaces, are left white, and are narrated. Poem source lines
  have one blank line after each wrapped source line and each poem file ends
  with eight blank lines. Total scroll length is `line_count * 12`.
- `bbb-native` localizes rich components, measures/wraps text, resolves the
  startup username, and projects immutable `HudWinScreen` or
  `HudDemoPopupScreen` values into the single `RendererFrame`. Screen
  precedence remains explicit; no protocol packet type or mutable input state
  crosses into `bbb-renderer`.
- `bbb-renderer` owns screen sanitization, exact pixel geometry, draw ordering,
  pack-backed title/edition/credits-vignette textures, and reuse of the
  existing end-portal and destination-multiply vignette pipelines. The poem
  path draws the 256x44 logo, 128x14 edition mark, 12-pixel text rows, and
  clamps the final line at screen center. Each new screen generation samples
  vanilla's one-in-10,000 `minceraft` title choice once; tests inject the sample
  rather than relying on nondeterminism.
- Opening a new win generation emits one client-owned
  `minecraft:music.credits` music start with zero delay and replacement of
  current music. Closing or replacing that generation emits one matching stop.
  These commands traverse the existing ordered render-effect/audio sink and
  sound registry; no parallel playback backend is introduced.
- Win/demo resource and texture candidates join the current transactional
  server-pack reload barrier. A rejected candidate preserves the previous
  published generation and active screen. Session reset removes retained
  screen state and client-owned music.

## Serial stages and verification

1. Add canonical screen/hint state, rich translatable chat+narration, native
   generation synchronization and input state, demo link confirmation, and
   single `PerformRespawn` completion. Focused checks:
   `bbb-world game_event`, `bbb-native game_event_screen`,
   `bbb-native chat_narration`, and existing protocol/net `client_command`
   tests. This state/input/network milestone receives a full gate.
2. Add the bounded pack text reader, exact poem/credits parsing and wrapping,
   startup-username projection, credits-music lifecycle, pack-backed direct
   textures, and transactional reload ownership. Focused checks:
   `bbb-pack win_screen`, `bbb-native win_screen_resources`,
   `bbb-native music`, and renderer/native `resource_reload`.
3. Add immutable HUD values and exact win/demo rendering, including portal
   background, logo/text/vignette, popup/menu geometry, focus/hover, and
   narration projection. Focused checks: `bbb-native hud_win_screen`,
   `bbb-native hud_demo_popup`, `bbb-renderer hud_win_screen`,
   `bbb-renderer hud_demo_popup`, and deterministic offscreen readback.
4. Mark packet coverage with exact evidence, remove the remaining feature-ledger
   TODO, archive this completed contract, and update the milestone index.
   Review the integrated diff, then run the full gate, playable fixture, and
   P0 audit.

Each stage is a buildable normal commit after focused tests, formatting, diff
check, docs lint, and review. A failed stage stops safely at the preceding clean
commit and is fixed forward without rewriting history. Stage 1 and the final
tip are explicit full-gate checkpoints; the four-commit rule may introduce an
earlier checkpoint if a stage needs multiple commits.
