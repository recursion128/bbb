# Multiplayer SpectatorGui menu closure

Status: completed engineering contract.

## Goal and completion

Close the vanilla 26.1 multiplayer `SpectatorGui` path without replacing it
with maintenance commands or the independent crosshair spectate shortcut.
Completion requires:

- spectator hotbar number keys and the default middle-mouse spectator action
  open, select, and activate the root/player/team menu;
- the root captures vanilla's eligible listed-player and team-member
  categories, player targets sort by profile UUID, spectator targets are
  excluded, and empty categories are disabled;
- the nine-slot, six-items-per-page menu implements previous/next/close slots,
  repeated-selection activation, wheel skipping, category prompts, and the
  three-second hold plus two-second fade/close lifecycle;
- player activation sends `ServerboundTeleportToEntityPacket` with the selected
  profile UUID, while the already independent crosshair path continues to send
  `ServerboundSpectateEntityPacket` with an entity id;
- the HUD substitutes the spectator hotbar for the item hotbar, draws the exact
  vanilla geometry, brightness/alpha, key labels, category/action text,
  player/team face icons, team color fill, and pack-backed spectator sprites;
- packet coverage, both duplicate feature-ledger entries, focused tests, the
  full workspace gate, the applicable playable smoke, and the P0 audit close at
  the final tip.

## Authority and non-goals

Authoritative sources are vanilla 26.1 `SpectatorGui`, `SpectatorMenu`,
`RootSpectatorMenuCategory`, `SpectatorPage`, `PlayerMenuItem`,
`TeleportToPlayerMenuCategory`, `TeleportToTeamMenuCategory`,
`PlayerFaceExtractor`, `Minecraft.handleKeybinds`, `MouseHandler.onScroll`,
`Gui.extractHotbarAndDecorations`, `ServerGamePacketListenerImpl`, and the two
serverbound packet classes.

This slice does not add spectator outline/shader configuration, a new in-game
key-binding screen, Social Interactions, or a server-side spectator
implementation. It does not alter the entity-id crosshair spectate range,
pickability, world-border, SetCamera, or camera-pose behavior already covered.
The existing control maintenance commands remain diagnostic surfaces and do
not become menu state owners.

## Frozen semantics and ownership

- `bbb-world` remains the canonical owner of active `PlayerInfo`, listed-player
  membership, scoreboard teams, rich team display names, and team colors. It
  adds only narrow read projections needed to build a menu catalog; no fade
  clock, selected slot, page, or renderer resource enters serializable world
  state.
- `bbb-native::input::spectator_menu` owns one cloneable simulation-side menu.
  Opening captures the same player/team membership set vanilla category
  constructors capture. Player entries retain their profile snapshot so a menu
  can still render and activate an entry after the active PlayerInfo map
  changes. Team activation uses its captured eligible members. A team icon
  chooses one captured eligible member when the root is built.
- The input owner stores an `Instant` selection anchor. Alpha is
  `clamp((5000ms - elapsed) / 2000ms, 0, 1)`: fully opaque through 3000ms,
  fading for 2000ms, and closed at zero. Production input/tick/render adapters
  call the same injected-time state-machine methods used by tests.
- The first hotbar number or spectator-action press creates the root without
  activating an item. A later number press selects that slot; selecting the
  same enabled slot again activates it. The spectator-action key activates the
  current selection, if any. Wheel input uses vanilla's negated primary step,
  skips empty and disabled slots without wrapping, and adjusts flying speed
  only while no spectator menu is active.
- Page item lookup uses `index = slot + page * 6`: slot 0 is previous on later
  pages, slot 7 is next (present but disabled when no next page), and slot 8 is
  close. Category selection resets slot/page. Player targets sort by UUID.
  Team iteration follows the deterministic canonical scoreboard collection;
  member presentation sorts by UUID when the team category opens.
- Player activation is the only menu action that emits a packet. It queues the
  existing UUID `TeleportToEntity` command after the same spectator/input
  guards as the menu. The server accepts it only for a spectator and answers
  through the already canonical player-position/level transition path.
  `SpectateEntity` remains the independent entity-id crosshair action.
- `bbb-native` resolves localization, rich team text, and `EntityPlayerSkin`
  handles while projecting one immutable `HudSpectatorGui` per frame.
  `bbb-renderer` owns sanitization, font measurement, pixel geometry, draw
  ordering, and texture handles.
- `bbb-renderer` derives an 8x8 face-plus-hat HUD sprite from each pack-backed
  default 64x64 player skin and each admitted dynamic skin. Dynamic loading and
  failure use the existing `EntityPlayerSkin` fallback/status rules. Default
  and dynamic face owners participate in the existing transactional resource
  reload and session invalidation paths.
- The renderer hotbar mode is explicit: ordinary, spectator (with an optional
  active menu page), or hidden. Spectator mode never leaks item hotbar icons or
  3D hotbar residency. No protocol packet type crosses into the renderer.

## Serial stages and verification

1. Add the native menu state machine, narrow world queries, hotbar/middle-mouse/
   wheel routing, expiry maintenance, and UUID packet activation. Focused
   checks: `bbb-world spectator`, `bbb-native spectator_menu`,
   `bbb-native spectator_mouse`, and existing spectator protocol/net command
   tests. This input/network milestone receives a full gate.
2. Add immutable native HUD projection, explicit renderer hotbar mode, exact
   spectator layout/sprites/text, pack/dynamic player-face owners, and
   resource-reload coverage. Focused checks: `bbb-native hud_spectator`,
   `bbb-renderer hud_spectator`, `bbb-native hud_assets`, and
   `bbb-renderer resource_reload`.
3. Mark the two spectator serverbound rows with exact evidence, remove both
   duplicate TODOs, archive this completed contract, and update the milestone
   index. Review the integrated diff, then run the full gate, playable fixture,
   and P0 audit.

Each stage is a buildable normal commit after focused tests, formatting, diff
check, docs lint, and review. A failed stage stops safely at the preceding clean
commit and is fixed forward without rewriting history.
