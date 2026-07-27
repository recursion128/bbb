# Camera Perspective Lifecycle

Status: completed on 2026-07-26.

## Goal and completion standard

Implement Vanilla 26.1's default F5 camera lifecycle as one native client
engineering slice:

- cycle `FIRST_PERSON -> THIRD_PERSON_BACK -> THIRD_PERSON_FRONT`;
- publish the final camera type from the simulation owner without a render-thread
  edge queue;
- derive the detached render camera from the current camera entity, including
  front-view mirroring, living scale / `CAMERA_DISTANCE`, a living mount's larger
  distance, and eight-ray `VISUAL` block clipping;
- keep gameplay picking rooted at the camera entity's eye and look direction;
- apply camera-type visibility to the camera entity, first-person hands/items,
  fishing-line origin, FOV item-use modifiers, and scoped-particle context; and
- implement the exact F5 post-effect transition while preserving independent F4
  toggles and every applied `SetCamera` refresh.

Completion requires deterministic focused tests for all three transitions,
F4/SetCamera interleaving, mirrored and clipped render poses, attribute/mount
distance, visual-shape exceptions, interaction-pose separation, entity
visibility, and first-person-only consumers. The ledger entry is then removed,
this contract is archived, the milestone index is updated, and final clean HEAD
passes the full gate and P0 audit.

## Scope and non-goals

Owned paths are native camera/input/runtime extraction, the existing entity
visibility and first-person presentation call sites, and narrow world queries
for camera distance and visual block clipping. The renderer continues to consume
the existing `CameraPose`; a detached pose stores the final world-space camera
position with `eye_height = 0`.

This slice does not add smooth-camera input, perspective persistence, an in-game
configuration UI, free camera, panoramic mode, nausea/bobbing changes, entity
collision in camera zoom, or resource-pack post graphs. Vanilla camera zoom
clips blocks only through `Level.clip(... Block.VISUAL, Fluid.NONE)`; it does not
use `CollisionGetter.getCollisions` and therefore does not clip against entities.

Existing first-person sleeping camera behavior is preserved. F5 remains blocked
while an existing native screen/input owner consumes gameplay keys, matching the
current input routing policy; this slice does not redesign screen focus.

Missing camera entities keep the current no-camera fallback. Missing or invalid
attribute data uses Vanilla defaults. Unloaded block cells remain misses, as with
the existing native block probes. Invalid/non-finite visual-clip inputs fail
closed to the requested distance rather than manufacturing a hit.

## Vanilla 26.1 evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `CameraType` defines `FIRST_PERSON(true, false)`,
  `THIRD_PERSON_BACK(false, false)`, and
  `THIRD_PERSON_FRONT(false, true)`; `cycle()` advances ordinal modulo three.
- `Minecraft.handleKeybinds` consumes each perspective click, cycles the option,
  calls `GameRenderer.checkEntityPostEffect(null)` only when leaving first
  person, calls it with the current camera entity only when returning, and
  invalidates level rendering. Back-to-front does not touch the post effect.
- `Minecraft.setCameraEntity` always calls `checkEntityPostEffect(cameraEntity)`
  independently of the current camera type. A `SetCamera` applied while in
  third person therefore refreshes selection and active state; later frames do
  not mask that selection merely because the camera remains detached.
- `Camera.setup` first interpolates the camera entity's eye pose. A detached
  front camera rotates to `(yRot + 180, -xRot)`, then moves backward by the
  clipped maximum of the living entity's `getScale * CAMERA_DISTANCE` and the
  same product for a living vehicle. Non-living defaults are scale 1 and
  distance 4.
- `Camera.getMaxZoom` casts eight rays from every `+/-0.1` XYZ corner toward the
  requested backward point with `ClipContext.Block.VISUAL` and
  `ClipContext.Fluid.NONE`; each hit shortens distance using the hit location's
  distance to the unjittered camera position.
- `BlockBehaviour.getVisualShape` defaults to collision shape. The complete
  Vanilla override set is `SoulSandBlock` and `MudBlock` (full block),
  `TransparentBlock`, `IronBarsBlock`, and `PowderSnowBlock` (empty),
  `FenceBlock` (outline shape), and `SnowLayerBlock` (the visible layer height).
- `Minecraft.pick` calls `LocalPlayer.raycastHitResult(partialTicks,
  cameraEntity)`. `LocalPlayer.pick` starts from `cameraEntity.getEyePosition`
  and its view vector, not the detached `Camera.position`.
- `LevelRenderer.extractVisibleEntities` hides the camera entity only when the
  camera is not detached and the entity is not sleeping. It still hides the
  local player when another entity is the active camera.
- `GameRenderer` renders in-hand items only for first person, awake, visible-GUI,
  non-spectator state. `FishingHookRenderer`, item-use FOV, and scoped-particle
  first-person branches also test `CameraType.isFirstPerson`.

## Boundaries and frozen integration API

- `camera_pose` owns a crate-private `CameraType` enum with `cycle`,
  `is_first_person`, and `is_mirrored`. `ClientInputState` owns one value,
  defaults it to first person, cycles it only on an ordinary F5 press edge, and
  includes only the final value in `RenderInputPublication`.
- `CameraPostEffectSelection` remains the last world selection token
  `(client-level generation, applied SetCamera count, generated entity effect)`.
  `ClientInputState` additionally owns the selected effect id. A changed world
  token replaces id and active state regardless of camera type. An F5
  first-person-boundary transition applies `checkEntityPostEffect` directly:
  leaving clears id/active, returning reselects the current camera entity and
  activates a supported id. Back-to-front preserves both fields. F4 flips only
  active state.
- Renderer-frame extraction and debug HUD read the paired selected id and active
  state from the immutable input publication. They no longer recompute the
  displayed id from world state after an F5 clear.
- Existing `camera_pose_from_world[_at_partial_tick]` remains the camera-entity
  pose used by gameplay picking. New
  `render_camera_pose_from_world_at_partial_tick(world, partial, camera_type)`
  applies mirroring and detached offset and is used by environment, scene,
  audio/listener, frustum, and renderer-frame extraction.
- `WorldStore::camera_entity_third_person_distance` is the narrow canonical
  query for living scale, `CAMERA_DISTANCE`, and living-vehicle maximum.
  `WorldStore::camera_visual_block_hit` is the narrow block-only segment query;
  it uses the existing collision-shape catalog plus the complete visual-shape
  override set and returns the nearest world-space hit.
- `EntityRenderVisibilityContext` carries `camera_detached`. It shows the camera
  entity when detached, retains the sleeping exception, and continues to hide
  the local player while another entity is the camera.
- Runtime extraction emits no first-person item/arm/map surfaces outside first
  person, selects the third-person fishing-line origin, passes `is_first_person`
  into camera FOV advancement and scoped-particle context, and continues to use
  the camera-entity pose for block/entity targeting.

## Serial stages, tests, and safe stops

1. **Canonical world geometry.** Add living camera-distance and block `VISUAL`
   segment queries, including the seven Vanilla override families and focused
   world tests.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-world camera_visual`.
2. **Simulation lifecycle.** Add camera type, F5 cycling, immutable publication,
   and paired post-effect selection/active ownership with transition and
   SetCamera/F4 ordering tests.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-native camera_perspective`.
3. **Render integration.** Add detached/mirrored/clipped pose derivation, split
   render pose from interaction pose, update visibility and first-person-only
   consumers, and cover the integrated extraction order.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native camera_perspective` and
   `scripts/cargo-dev.sh test -p bbb-native camera_pose`.
4. **Closeout.** Remove the F5 ledger entry, archive this contract, append the
   milestone index, run affected focused tests and all per-commit checks.

Each stage ends in a buildable normal commit. The contract is a safe stop before
code; the world API, simulation lifecycle, and render integration commits are
subsequent safe stops. The final closeout runs `scripts/cargo-dev.sh gate`
followed by `scripts/audit-p0.sh`. Failures are repaired forward from the
preceding clean stage. The latest fully gated baseline before this engineering
slice is `5cbfd95ae`.

## Hardest-ready decision

This is the hardest ready bounded candidate after built-in camera post-effect
execution. It crosses canonical world geometry and attributes, simulation input
ordering, immutable publication, camera math, scene visibility, first-person
presentation, and post-effect lifecycle, while Vanilla freezes every transition
and fallback.

The selected-pack post-graph candidate remains harder in raw shader scope but is
not ready: the repository has no frozen GLSL compilation boundary or complete
malformed/uncompilable override policy. The broad survival movement family and
Tutorial pickup partial likewise lack a single bounded integration contract.
Creative-tab player preview and narrower presentation items are ready but have
less state-machine, cross-boundary, and verification risk.
