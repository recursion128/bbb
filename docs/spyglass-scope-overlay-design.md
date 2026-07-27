# Spyglass Scope Overlay

Status: active engineering slice.

## Goal and completion standard

Implement Vanilla 26.1's first-person spyglass scope as one native client
engineering slice:

- derive one canonical local-player `isScoping` query from active item use and
  the held spyglass protocol id;
- advance `Gui.scopeScale` from a render-frame game-time delta owner rather
  than simulation partial ticks;
- load `textures/misc/spyglass_scope.png` from the selected resource-pack
  generation and include it in the renderer reload checkpoint;
- submit the centered scope texture and four black outside-mask rectangles in
  the existing HUD pass before the crosshair; and
- preserve Vanilla's first-person, hidden-GUI, and reset behavior.

Completion requires deterministic focused tests for the local scoping query,
frame-delta math, every `scopeScale` transition, wide/tall viewport geometry,
texture/mask/crosshair ordering, hidden-GUI suppression, selected-pack loading,
and reload rollback ownership. The ledger entry is then removed, this contract
is archived, the milestone index is updated, and final clean HEAD passes the
full gate and P0 audit.

## Scope and non-goals

This slice owns the narrow world scoping query, native HUD frame timing and
renderer-frame extraction, HUD resource loading, renderer resource ownership,
and scope draw geometry. `HudRenderRuntime` remains the render-thread owner of
the interpolated scale; no scope edge or mutable GUI state enters
`WorldStore` or the immutable simulation publication.

The existing first-person item suppression, spyglass FOV, sounds, player model
pose, and scoped-particle behavior stay in place and switch to the canonical
world query where they currently duplicate the same test.

This slice does not implement equippable camera overlays, portal/nausea/powder
snow overlays, GUI-scale configuration, a new render pass or GPU pipeline,
scope post-processing, zoom input, or in-game configuration UI. It does not
draw the scope in third person, for another camera entity's item use, or while
the HUD is hidden.

The first render frame has no prior elapsed sample and therefore advances the
scale by zero. Invalid or non-positive elapsed/tick-interval inputs also advance
by zero. Otherwise the game-time delta is deliberately not capped: Vanilla
passes `Timer.deltaTicks` directly to `Mth.lerp`.

## Vanilla 26.1 evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `net/minecraft/client/gui/Gui.java` owns `scopeScale`, initialized to zero.
  `extractRenderState` calls camera-overlay extraction only when the GUI is not
  hidden, before crosshair extraction and before the next HUD stratum.
- `Gui.extractCameraOverlays` first advances
  `scopeScale = Mth.lerp(0.5F * gameTimeDeltaTicks, scopeScale, 1.125F)`.
  It then draws the scope only for a first-person camera whose local player is
  scoping. First-person non-scoping resets the value to `0.5F` after the
  interpolation. Third person still advances the value but neither draws nor
  resets it. Hidden GUI performs none of those operations.
- `Gui.extractSpyglassOverlay` uses the smaller GUI dimension as a square,
  multiplies it by `scopeScale`, floors the width and height, centers the
  texture with integer division, then fills the areas below, above, left, and
  right with opaque black.
- `net/minecraft/client/DeltaTracker.java` computes `Timer.deltaTicks` as elapsed
  milliseconds divided by the current target milliseconds per tick.
  `getGameTimeDeltaTicks` returns that per-frame value; it is distinct from the
  residual partial tick and from the separately capped real-time delta.
- `net/minecraft/world/entity/player/Player.java` defines `isScoping` as
  `isUsingItem() && getUseItem().is(Items.SPYGLASS)`.

## Boundaries and frozen integration API

- `bbb-world` exposes `WorldStore::local_player_is_scoping() -> bool`. It reads
  the canonical local player's active-use flag and using-hand item protocol id.
  Missing player/use state or an unknown item id returns `false`.
- `HudRenderRuntime` owns one previous-HUD-frame timestamp and `scope_scale`.
  One frame sample produces both game-time delta
  `elapsed / publication.tick_interval` and the existing real-time delta
  `elapsed * 20`; Win-screen presentation consumes the latter and scope
  interpolation consumes the former. The timestamp advances exactly once per
  render frame.
- `HudRenderRuntime::spyglass_scope` receives game-time delta, `hide_gui`,
  camera `is_first_person`, and canonical `is_scoping`. It reproduces Vanilla's
  update/draw/reset order and returns an optional immutable
  `bbb_renderer::HudSpyglassScope { scale }` for that frame.
- `bbb-native::hud_assets` loads the scope through the existing selected-stack
  standalone GUI texture path. `bbb-renderer` owns the resulting GPU image,
  moves it through the exhaustive HUD resource reload checkpoint, and exposes
  narrow upload and per-frame scope setters.
- `RendererFrame` carries only the optional per-frame scope value. The renderer
  combines it with its resource owner and emits the scope texture plus clipped
  opaque-black mask rectangles in physical HUD coordinates. It emits those
  commands before crosshair commands and suppresses them when `hide_gui` is
  true even if a stale frame value is presented.
- Scope drawing reuses the existing HUD sprite batch, white-pixel solid-quad
  resource, render pass, and alpha blend contract. There is no new persistent
  pipeline, bind-group layout, or world/runtime mutation boundary.

## Serial stages, tests, and safe stops

1. **Canonical state and frame clock.** Add the world scoping query, replace
   duplicate local-scoping tests where appropriate, factor one HUD frame-delta
   sample, and implement Vanilla's scope-scale lifecycle.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world local_player_is_scoping` and
   `scripts/cargo-dev.sh test -p bbb-native spyglass_scope`.
2. **Resource and renderer integration.** Add selected-pack texture loading,
   reload checkpoint ownership, renderer-frame handoff, exact centered geometry,
   black mask, and HUD ordering.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-renderer spyglass_scope` and
   `scripts/cargo-dev.sh test -p bbb-native spyglass_scope`.
3. **Closeout.** Remove the ledger item, archive this contract, append the
   milestone index, run all affected focused tests and per-commit checks.

Each stage ends in a buildable normal commit. This committed contract is the
first safe stop; canonical timing and render integration are subsequent safe
stops. The final closeout runs `scripts/cargo-dev.sh gate` followed by
`scripts/audit-p0.sh`. Failures are repaired forward from the preceding clean
stage. The latest fully gated baseline before this engineering slice is
`1e0cf3ec5`.

## Hardest-ready decision

This is the hardest ready bounded presentation candidate after camera
perspective closure. It crosses canonical item-use state, render-frame timing,
selected-pack resources, reload rollback ownership, renderer-frame extraction,
and HUD command ordering, while Vanilla freezes its formulas and fallbacks.

The selected-pack post-graph candidate remains harder in raw shader scope but
is not ready: the repository still has no frozen GLSL compilation boundary or
complete malformed/uncompilable override policy. Broad survival movement does
not yet form one bounded integration contract. Creative-tab player preview is
ready but has less state-machine and reload-lifecycle risk.
