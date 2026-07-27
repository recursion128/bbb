# Player-list presentation closure

Status: active engineering contract.

Pre-start fully gated baseline: `9870f9a5ac537c489135e15fceb28ca833466738`.

## Goal and completion

Close the remaining vanilla 26.1 multiplayer player-list presentation path.
Completion requires:

- the net owner publishes whether the current play session uses encrypted
  transport, and native carries that connection-scoped value through the
  immutable simulation publication;
- a false-to-true player-list visibility edge immediately narrates the sorted,
  decorated, at-most-80 displayed names through the system narrator;
- remote clients show player faces only for encrypted sessions, including
  selected profile skins, the synchronized hat-part flag, and the exact loaded
  Dinnerbone/Grumm vertical flip;
- all six vanilla ping sprites replace the synthesized bar drawing with exact
  thresholds, dimensions, placement, selected-pack loading, and atomic reload
  rollback;
- focused tests, the feature ledger, archive index, final full gate, and P0
  audit close at the final tip.

## Authority and non-goals

Authoritative sources are vanilla 26.1 `PlayerTabOverlay`,
`PlayerFaceExtractor`, `AvatarRenderer`, `LivingEntityRenderer`,
`ComponentUtils`, `Connection`, and the six
`assets/minecraft/textures/gui/sprites/icon/ping_*.png` resources.

The native client has no integrated-server owner, so
`minecraft.isLocalServer()` is permanently false and heads depend only on
transport encryption. This slice does not add an integrated server, change
player-info wire formats, redesign profile-skin downloads, implement the
creative inventory player preview, or add in-game narrator/configuration UI.
Existing player ordering, team decoration, score layout, heart presentation,
header/footer wrapping, and the 80-entry limit remain the single source of
truth and are extended rather than duplicated.

## Frozen semantics and ownership

- `bbb-net` adds `NetEvent::PlaySessionSecurity { encrypted }`. The event
  stream emits it immediately before the matching
  `NetEvent::Play(PlayClientbound::Login)` and derives the value only from
  `RawConnection::encryption_enabled()`. Login/reconfiguration on one
  connection restates the value for each play session.
- `bbb-native::SimulationCore` owns one `play_session_encrypted` boolean.
  `Connected`, `Disconnected`, and receiver closure reset it to false;
  `PlaySessionSecurity` replaces it. `SimulationPublication` copies the value
  as immutable presentation input. Neither `WorldStore` nor `NetCounters`
  becomes transport-security authority.
- Native player-list extraction receives the published boolean and sets one
  list-wide `show_heads` flag. Each entry carries its existing profile UUID
  plus the selected `EntityPlayerSkin`, synchronized `show_hat`, and a flip
  flag. The flip is true only when a player entity with that UUID is currently
  loaded and the profile name is exactly `Dinnerbone` or `Grumm`; unlike the
  world entity model, this direct `AvatarRenderer.isPlayerUpsideDown` call has
  no cape-part gate.
- `bbb-world` exposes only the narrow loaded-player-entity-by-profile query
  needed for that flip gate. Player identity and entity lifecycle remain
  canonical world state; transport security does not cross into world.
- `bbb-renderer` extends the existing default/dynamic HUD face residency with
  paired base-only and base-plus-hat 8x8 owners. Spectator/social callers keep
  requesting the hat-composited face. Player-list draws reserve nine pixels
  only when `show_heads`, draw the 8x8 face at the row origin, advance the name
  origin by nine, and vertically reverse the face UV for the flip flag.
- A typed six-value ping-sprite catalog owns the latency mapping:
  negative=`ping_unknown`, `<150`=`ping_5`, `<300`=`ping_4`,
  `<600`=`ping_3`, `<1000`=`ping_2`, otherwise=`ping_1`. Native derives
  selected-pack paths from that catalog; renderer draws the resolved 10x8
  sprite at the original slot origin plus `slotWidth - 11`, never a bar
  fallback.
- Native reuses the same visibility predicate and sorted decorated entry
  projection for rendering and narration. A render-thread boolean records the
  previous visibility even when narration is disabled. On false-to-true only,
  it joins the flattened displayed names with `", "` and calls
  `say_system_now` with localized
  `multiplayer.player.list.narration` (fallback `Online players: %s`).
- The world-to-renderer path continues through the single `RendererFrame`
  handoff. Ping resources and paired face owners participate in the existing
  exhaustive HUD reload checkpoint; old/new resource generations cannot mix.

## Serial stages and verification

1. Add the typed net security event, runtime connection owner, immutable
   publication field, and exact event ordering/reset tests. Focused checks:
   `bbb-net play_session_security` and `bbb-native
   play_session_security`. This crosses transport/publication architecture and
   receives a full gate before presentation work.
2. Centralize player-list entry/visibility projection, add immediate opening
   narration, the loaded-player query, head state, paired face residency, and
   exact encrypted/hat/flip layout tests. Focused checks: `bbb-world
   player_list`, `bbb-native hud_player_list`, and `bbb-renderer
   hud_player_list`.
3. Add the derived ping catalog, selected-pack loading, GPU owners, exact
   sprite draw tests, and reload checkpoint/rollback coverage. Focused checks:
   `bbb-native hud_assets`, `bbb-renderer hud_player_list`, and
   `bbb-renderer resource_reload`.
4. Remove the completed ledger item, archive this contract, review the
   integrated diff, then run the full gate and P0 audit.

Every stage is a buildable normal commit after its focused tests,
`cargo fmt --all --check`, `git diff --check`, documentation lint when
applicable, and full diff review. Run a full gate after stage 1, at any
four-commit checkpoint, and at the final tip. A failure stops safely at the
preceding clean commit and is fixed forward without rewriting history.
