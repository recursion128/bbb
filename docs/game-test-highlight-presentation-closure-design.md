# GameTest Highlight Presentation Closure Design

Status: active engineering contract.

## Goal and completion standard

Close `play/clientbound/CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS` as one
user-visible vertical slice. The existing wire decoder, unique net dispatch,
and last-packet diagnostic remain valid. This engineering slice adds the
missing renderer-owned marker behavior:

- retain every live marker by absolute block position and refresh the same
  position rather than appending a duplicate;
- expire markers after the exact 10-second monotonic wall-clock interval and
  clear them at the client-level generation boundary;
- render the padded translucent green cuboid through the existing
  `debugFilledBox` surface;
- render the relative-position short string as centered, full-bright white
  billboard text at the vanilla position and scale;
- give that text a real late always-on-top path after transparency composition;
- change both partial packet-coverage cells to `complete`, remove the renderer
  TODO, archive this contract, and pass the full gate plus applicable
  playable/P0 checks.

The completion evidence must include multiple absolute positions, same-key
refresh, the strict expiry boundary, level reset, exact geometry/color/text,
the standard-versus-always-on-top renderer split, and the late depth-clear
pass ordering.

## Scope and non-goals

Owned boundaries are:

- `bbb-world`: level-bound active marker state, exact refresh/expiry semantics,
  query/counter surface, and injected monotonic receive time;
- `bbb-native`: simulation expiry cadence and projection into the existing
  debug filled-box/text submission surfaces;
- `bbb-renderer`: an explicit `always_on_top` text property, separate upload
  owners, and the late pass after transparency composition;
- protocol coverage overrides/catalog tests and closeout documentation.

This slice does not add a debug toggle, in-game configuration UI, GameTest
authoring controls, test-instance block presentation, or any other remote debug
subscription renderer. The packet is unconditional in vanilla and therefore
does not depend on F3/debug-overlay visibility.

The existing `last_game_test_highlight_pos` diagnostic is retained for
compatible control snapshots, but it is not the presentation owner. Active
markers are level-bound and do not survive login, reconfiguration, respawn
dimension replacement, or an explicit level clear.

Missing font-atlas state may suppress only the text through the renderer's
existing guarded text path; the cuboid remains independently drawable. Invalid
GPU submissions continue to use the existing sanitizer behavior.

## Authoritative vanilla 26.1 facts

Local sources:

- `ClientboundGameTestHighlightPosPacket.STREAM_CODEC` decodes `absolutePos`
  followed by `relativePos`, both as `BlockPos.STREAM_CODEC`.
- `ClientPacketListener.handleGameTestHighlightPos` immediately calls
  `LevelRenderer.gameTestBlockHighlightRenderer.highlightPos(absolute,
  relative)` on the client packet thread.
- `GameTestBlockHighlightRenderer.highlightPos` converts the relative position
  with `Vec3i.toShortString`, then replaces the `HashMap` entry keyed by
  absolute position with color `1610678016` (`0x6000ff00`) and
  `Util.getMillis() + 10000`.
- `emitGizmos` removes only entries for which `time > removeAtTime`; a marker
  remains visible exactly at the deadline.
- `LevelRenderer.setLevel` clears the marker renderer for every level
  replacement, including `null`.
- `renderMarker` calls `Gizmos.cuboid(pos, 0.02F,
  GizmoStyle.fill(0x6000ff00))`. `Gizmos.cuboid(BlockPos, padding, ...)`
  constructs `new AABB(pos).inflate(padding)`, giving
  `[x-0.02,y-0.02,z-0.02]` through `[x+1.02,y+1.02,z+1.02]`.
- ARGB `0x6000ff00` maps to renderer RGBA `[0, 255, 0, 96]`.
- Non-empty text is exactly `"{x}, {y}, {z}"`, positioned at
  `(absolute.x+0.5, absolute.y+1.2, absolute.z+0.5)`, centered, white,
  full-bright, and scaled to `0.16`.
- The text gizmo calls `setAlwaysOnTop`. `LevelRenderer.addLateDebugPass`
  executes after the transparency post chain, clears the main depth texture to
  `1.0`, and renders the always-on-top primitive group before later GUI and
  first-person presentation.
- Standard game-test cuboids stay in the normal gizmo group. They use ordinary
  scene depth, alpha blending, and the existing translucent-quad sort path.

## Frozen ownership and integration contract

`ClientDebugGameState` gains a level-bound ordered map keyed by absolute
`BlockPos`. Each value owns the relative position and
`remove_at_monotonic_millis`. Packet application:

1. increments the existing packet counter;
2. records the compatible last-packet diagnostic;
3. inserts/replaces exactly one map value using the injected client monotonic
   receive time plus 10,000 milliseconds;
4. updates a tracked-marker counter.

`PlayApplyEffects` exposes a client monotonic millisecond callback. Native
online dispatch supplies the process monotonic clock; state-only callers use a
deterministic zero default and can exercise the explicit `_at` method.
Simulation expiry runs after the single network drain and removes only markers
whose deadline is strictly less than the sampled time. Level-bound cleanup
clears both active markers and the last-position diagnostic.

Native render extraction appends game-test submissions after ordinary/remote
debug gizmos, matching `DebugRenderer.emitGizmos` followed by
`GameTestBlockHighlightRenderer.emitGizmos`. One active marker projects to:

- one `DebugFilledBox` with the exact inflated bounds and RGBA color;
- one `DebugTextBillboard` with exact short text, position, white color,
  `scale=0.16`, centered alignment, and `always_on_top=true`.

`DebugTextBillboard::centered` defaults `always_on_top` to false, preserving all
existing callers. Standard and always-on-top text use distinct per-frame GPU
buffers so queue writes cannot alias before command submission. The standard
pass keeps its current scene depth and location. The late pass:

- runs after `transparency_combine_pass` and before
  `transparency_blit_pass`;
- loads the composed final color target;
- clears the shared scene depth to `1.0`;
- draws only always-on-top text with the existing font/lightmap bindings and
  text pipeline.

No packet counter, control snapshot, or debug toggle may be used as the render
source when the active marker map is empty.

## Serial stages, focused tests, and checkpoints

1. **Canonical lifecycle.** Add the active marker map, injected receive time,
   strict expiry and level cleanup; wire native simulation expiry. Focused
   commands:
   `scripts/cargo-dev.sh test -p bbb-world game_test_highlight` and
   `scripts/cargo-dev.sh test -p bbb-native game_test_highlight`.
2. **Projection and late GPU path.** Extend the renderer text contract, add
   separate standard/always-on-top uploads and the late depth-clear pass, then
   project exact marker cuboids and labels from native. Focused commands:
   `scripts/cargo-dev.sh test -p bbb-renderer debug_text` and
   `scripts/cargo-dev.sh test -p bbb-native game_test_highlight`.
3. **Coverage closeout.** Mark canonical/runtime coverage complete, regenerate
   the checked-in packet catalog, delete the renderer TODO, archive this
   contract, and add the milestone index entry. Run packet coverage, generator
   check, docs lint, formatting, and diff checks.

Every stage is a buildable normal commit. The contract plus three stages reach
the four-commit engineering checkpoint, so the final tip runs
`scripts/cargo-dev.sh gate`, the available playable modes, and
`scripts/audit-p0.sh`.

Failures are fixed forward. Before the final renderer stage, the canonical map
is observable but has no new drawing path. After the renderer stage, absence of
markers or font resources safely yields no text draw without inventing stale
state.

## Hardest-ready decision

The broader OpenScreen completion and client game-event presentation live under
the ledger's `Deferred` section, so they are excluded until their restart
conditions are met. The remaining broad physics/performance inventories are not
single frozen semantic targets, and equal-load official-client measurement is
blocked by the unavailable Java executable. GameTest highlight is therefore
the hardest current ready packet closure with a complete, testable integration
contract: it crosses canonical timed state, level lifecycle, render extraction,
translucent gizmos, font billboards, depth ownership, and frame-graph ordering.
