# World-Border Vignette Closure Design

Status: closed engineering contract.

## Goal and completion standard

Close the six partial Play clientbound world-border packet families as one
user-visible vertical slice:

- `minecraft:initialize_border`
- `minecraft:set_border_center`
- `minecraft:set_border_lerp_size`
- `minecraft:set_border_size`
- `minecraft:set_border_warning_delay`
- `minecraft:set_border_warning_distance`

The existing protocol decode, unique net dispatch, canonical `WorldBorderState`,
tick lerp, interaction boundary, and 3D forcefield remain authoritative. This
engineering slice adds the missing `Gui.extractVignette` consumer: tick the
camera-local vignette brightness, derive the world-border warning tint each
frame, load `textures/misc/vignette.png` from the selected pack generation, and
draw it before the ordinary HUD through the exact destination-multiplying
vignette blend.

Completion requires:

- focused semantic tests for brightness smoothing, border distance and moving
  threshold, warning tint, missing-camera behavior, hide-GUI gating, and
  connection/level continuity;
- renderer tests for the full-surface draw, linear texture sampling, exact
  `ZERO` / `ONE_MINUS_SRC_COLOR` RGB blend and preserved destination alpha;
- selected-pack and transactional reload tests for the vignette texture owner;
- all six packet coverage rows changed from `partial` to `complete`, the
  world-border warning todo removed, and the contract archived;
- a clean final HEAD passing the full gate and the applicable playable/P0
  audits.

## Scope and non-goals

Owned paths are the existing world-border state in `bbb-world`, a new focused
vignette projection owner in `bbb-native`, the renderer camera-overlay owner,
HUD/pack asset loading, reload checkpoint ownership, packet-coverage evidence,
and closeout documentation.

This slice does not add an in-game option or configuration screen. It follows
the vanilla default `Options.vignette = true`; F1 `hideGui` suppresses the draw
but does not pause brightness ticking. It does not implement spyglass,
equippable, powder-snow, portal, nausea, or other camera overlays, and it does
not change the already-complete 3D world-border forcefield.

Missing/unloaded camera light data uses a full-bright fallback, preventing a
transient unloaded chunk from darkening the whole screen. A missing vignette
asset leaves the draw owner empty and emits the existing resource-load
diagnostic path; a failed selected-pack reload restores the prior texture owner.

## Vanilla 26.1 contract

Authoritative local sources:

- `Gui.extractRenderState` and `Gui.extractCameraOverlays`: the vignette is the
  first camera overlay, is inside the `!Options.hideGui` branch, and precedes
  crosshair/hotbar HUD extraction.
- `Gui.tick` and `Gui.updateVignetteBrightness`: every raw client tick with a
  camera samples the block position at `(camera.x, camera.eyeY, camera.z)`,
  computes `Lightmap.getBrightness(dimensionType, maxLocalRawBrightness)`, then
  updates `vignetteBrightness += ((1 - levelBrightness) -
  vignetteBrightness) * 0.01`.
- `Gui.extractVignette`: `distToBorder` uses the current camera X/Z and
  partial-tick-zero border bounds. The moving threshold is
  `min(getLerpSpeed() * warningTime,
  abs(getLerpTarget() - getSize()))`; warning distance is its maximum with
  `warningBlocks`; strength is `1 - distance / warningDistance` when the
  distance is smaller, then clamped to `[0, 1]`.
- The non-warning tint is opaque `(brightness, brightness, brightness)`. A
  warning uses red `brightness * (1 - strength)` and green/blue
  `brightness + (1 - brightness) * strength`.
- `WorldBorder.MovingBorderExtent.getLerpSpeed` is
  `abs(from - to) / duration`; `getSize` is the tick-updated current size.
  `WorldBorder.getDistanceToBorder` takes the minimum distance to the four
  partial-tick-zero edges.
- `Lightmap.getBrightness` maps raw light `v = level / 15` through
  `v / (4 - 3v)`, then lerps from the dimension's `ambient_light` to `1`.
- `RenderPipelines.VIGNETTE` uses RGB source factor `ZERO` and destination
  factor `ONE_MINUS_SRC_COLOR`, while destination alpha is preserved. The
  texture metadata enables blur, so the native sampler is linear.

## Boundaries and frozen integration API

- `bbb-world` remains the only owner of border packet state and queries. No
  second border clock or packet switch is introduced.
- `bbb-native::runtime::vignette` owns the mutable GUI tick value and the pure
  world/camera projection. It consumes `WorldStore`, the existing camera pose,
  raw client-tick count, and `hideGui`; its only renderer-facing result is an
  optional opaque RGB `HudVignette` value.
- `RendererFrame` carries that optional value and
  `apply_renderer_frame` replaces the renderer's latest vignette state once per
  publication.
- `bbb-renderer::vignette` owns the public `HudVignette`, the texture/bind
  group/uniform, the exact blend pipeline, and the pre-HUD surface pass.
- `bbb-native::hud_assets` resolves
  `minecraft:textures/misc/vignette.png` from the selected resource stack and
  uploads it through one renderer method. The renderer reload checkpoint owns
  the texture/bind-group pair atomically.

No fallback may infer border state from packet counters, the 3D wall render
state, or protocol ids.

## Serial stages, tests, and checkpoints

1. **Semantic projection.** Add the renderer-facing value and the native
   vignette tick/projection owner, wire it through `RendererFrame`, and test the
   vanilla formulas, tick cadence, camera absence, F1 gating, and lerp
   threshold. Focused command:
   `scripts/cargo-dev.sh test -p bbb-native vignette`.
2. **Asset and GPU presentation.** Load the pack texture, add the exact
   fullscreen blend pass before ordinary HUD, and include the owner in
   transactional reload rollback. Focused commands:
   `scripts/cargo-dev.sh test -p bbb-renderer vignette` and
   `scripts/cargo-dev.sh test -p bbb-native vignette_texture`.
3. **Coverage closeout.** Update all six sparse packet overrides, regenerate
   the checked-in catalog, delete the unsupported todo, archive this contract,
   and add the milestone index entry. Run packet coverage, generator check,
   docs lint, formatting, and diff checks.

Each stage is a buildable, reviewable normal commit. The final closeout is the
fourth commit including this contract, so the engineering checkpoint runs
`scripts/cargo-dev.sh gate` at the final tip. Then run `playable-gate.sh` in the
available modes and `scripts/audit-p0.sh`.

If a stage fails, stop at the preceding clean commit and fix forward. The
renderer skips the pass when either state or texture is absent, and resource
reload rollback restores the complete previous owner, so no stage needs a
half-applied compatibility mode.

## Hardest-ready decision

This family outranks the single-packet GameTest marker because it closes six
P1 ledger rows and crosses canonical moving state, raw client ticks,
camera/light sampling, resource generations, and a nonstandard GPU blend.
Client game-event presentation is broader, but its ledger entry is explicitly
under `Deferred`, so it is not a ready candidate.
