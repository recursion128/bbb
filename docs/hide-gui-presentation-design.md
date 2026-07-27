# F1 Hide-GUI Presentation Parity

Status: active engineering contract.

## Goal and completion standard

Close the remaining F1-backed `hideGui` presentation gap as one renderer-input
slice. The existing edge-triggered F1 input state remains authoritative. Every
currently implemented presentation consumer must follow the Vanilla 26.1
visibility split:

- hide the ordinary in-game GUI, including the normal/debug crosshair, hotbar
  and spectator bar, status rows, contextual/experience/locator bar, mob
  effects, boss bars, action bar, title, scoreboard, chat, and player list;
- hide the block selection outline, generic entity names, vignette, and the
  complete first-person hand/item/map pass;
- keep the sleep overlay, open screens, F3 debug overlay, system/advancement/
  recipe toasts, and world/debug rendering visible; and
- restore every retained presentation owner immediately when F1 is toggled
  back, without requiring a world, packet, resource, or screen-state change.

Completion requires focused renderer and native tests for both sides of that
split, removal of the unsupported-feature todo, an archived contract and
milestone index entry, a clean final HEAD, and the full gate plus P0 audit.

## Scope and non-goals

Owned paths are the existing `bbb-native` input-to-renderer frame publication,
the `bbb-renderer` HUD, first-person, selection-outline, and item-residency draw
boundaries, focused tests, and closeout documentation.

This slice does not add an in-game configuration UI or persist F1 state.
It does not implement presentation that is already absent, including
equippable/powder-snow camera overlays, the spyglass scope, sound subtitles, or
the item-activation animation. Those features must consume the same renderer
visibility owner when implemented. It does not hide full-screen game/screen
UIs, toasts, the sleep fade, F3 diagnostics, particles, block-destroy progress,
debug gizmos, or glowing-entity outlines.

## Vanilla 26.1 evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `Gui.extractRenderState` wraps camera overlays, crosshair, hotbar/decorations,
  effects, boss bars, demo overlay, scoreboard, overlay message, title, chat,
  and tab list in `!Options.hideGui`. `extractSleepOverlay` sits between the two
  hidden groups and is unconditional. Its hide-GUI branch still defers sound
  subtitles for an in-game UI screen.
- `GameRenderer.extractGui` extracts the active `Screen`, saving indicator,
  `ToastManager`, and `Gui.extractDebugOverlay` outside
  `Gui.extractRenderState`; F1 therefore does not hide screens, toasts, or F3.
- `GameRenderer.renderItemInHand` requires first-person camera, awake player,
  non-spectator mode, and `!optionsRenderState.hideGui`.
- `GameRenderer.shouldRenderBlockOutline` requires
  `!minecraft.options.hideGui`; this controls the normal targeted-block
  outline, not independent debug renderers.
- `GameRenderer.renderLevel` gates the F3 three-dimensional crosshair on
  `!optionsState.hideGui`.
- `ScreenEffectRenderer.renderScreenEffect` keeps inside-block, water, and fire
  effects independent from F1 but gates only the item-activation animation on
  `!hideGui`. These effects are not yet native presentation owners in `bbb`.

## Boundaries and frozen integration API

- `ClientInputState` remains the only mutable F1 owner. No duplicate toggle or
  world state is introduced.
- `RendererFrame` snapshots `hide_gui` once with the rest of the post-input
  render state. `apply_renderer_frame` calls one renderer setter before item
  residency is resolved.
- `bbb-renderer::Renderer` retains all latest HUD, screen, toast, and
  first-person data while hidden. Its `hide_gui` draw policy suppresses only
  ordinary in-game HUD commands/hotbar residency and the first-person pass.
  This retained-state contract is required so F1-off restores unchanged chat,
  boss, hotbar, screen, and item state immediately.
- `bbb-native` continues to gate the vignette and generic name tags at their
  existing extraction owners. It additionally suppresses only the normal
  selection outline. Debug outlines and gizmos remain independent.
- Advancement and recipe Toast projection is unconditional with respect to
  F1. Their runtime clocks and sounds already advance independently and remain
  unchanged.

The renderer flag is the reusable visibility contract for future ordinary HUD
or first-person draw owners. Missing assets retain their existing no-draw
fallback; `hide_gui` never clears or rebuilds a resource owner.

## Serial stages, tests, and safe stops

1. **Renderer visibility owner.** Add `Renderer::set_hide_gui`, gate ordinary
   HUD commands, ordinary hotbar item residency, and the complete first-person
   pass while retaining screens, sleep, debug, and Toast data.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-renderer hide_gui`.
2. **Native frame integration.** Publish the post-input flag before residency,
   gate the normal selection outline, retain existing name/vignette gates, and
   remove the incorrect advancement/recipe Toast gates.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-native hide_gui`.
3. **Closeout.** Remove the unsupported todo, archive this contract, append the
   engineering milestone, and run the per-commit checks.

Each implementation stage is a buildable normal commit. The final closeout
runs `scripts/cargo-dev.sh gate` and `scripts/audit-p0.sh`. A failure stops at
the preceding clean commit and is repaired forward. Because the renderer
retains all hidden state, there is no partial migration or compatibility mode.

## Hardest-ready decision

This slice is the hardest ready candidate after the packet, registry, and item
component audits found no non-deferred partial families. The remaining
`SetPlayerInventory` side effect requires the full Tutorial step state machine,
tag-driven item transitions, Tutorial Toast presentation, startup/persistence
ownership, and additional non-packet inputs; that integration contract cannot
yet be frozen as a packet-local slice. F1 parity already has an authoritative
input owner and bounded renderer consumers, but crosses retained HUD state,
GPU pass ordering, residency, world outlines, and Toast exceptions.
