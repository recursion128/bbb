# Sprint-Swim Camera Presentation Design

Engineering contract for closing the remaining local-player sprint/swim camera
and presentation ledger item without reopening the already completed movement
physics or player-model animation owners.

## Goal and completion standard

The native client must present the locally controlled player like Vanilla 26.1
across sprint and swimming transitions:

- the sprinting movement-speed modifier contributes to
  `AbstractClientPlayer.getFieldOfViewModifier`, including while sprint-swimming;
- `Camera.tick` eases its retained eye height halfway toward the active camera
  entity's current pose eye height and render extraction interpolates the two
  tick endpoints;
- `GameRenderer.bobView` consumes the existing player `ClientAvatarState`
  walk-distance and bob values after hurt tilt and before portal/nausea
  distortion; and
- swimming suppresses new view-bob input through the existing
  `AbstractClientPlayer.updateBob` gate, so the retained bob decays instead of
  snapping or continuing a ground-walk pulse.

Completion requires focused world/native/renderer tests, deterministic matrix
evidence for transform order and swimming suppression, deletion of the one
remaining sprint-swim ledger TODO, an archived contract and milestone entry, a
clean reviewed diff, and a fully gated final HEAD.

## Authoritative 26.1 references

Minecraft sources under `~/Work/mc-code/sources/26.1/`:

- `net.minecraft.client.Camera`
- `net.minecraft.client.renderer.GameRenderer`
- `net.minecraft.client.player.AbstractClientPlayer`
- `net.minecraft.client.player.LocalPlayer`
- `net.minecraft.client.entity.ClientAvatarState`
- `net.minecraft.world.entity.Entity`
- `net.minecraft.world.entity.player.Player`

The exact rules are:

- `Camera.tick`: save `eyeHeightOld`, then
  `eyeHeight += (entity.getEyeHeight() - eyeHeight) * 0.5`.
- `Camera.alignWithEntity`: add
  `lerp(partialTick, eyeHeightOld, eyeHeight)` to the interpolated entity
  position before detached-camera clipping.
- `AbstractClientPlayer.getFieldOfViewModifier`: use the effective
  `MOVEMENT_SPEED` attribute, which already contains the transient sprinting
  modifier, divided by ability walking speed.
- `ClientAvatarState.getBackwardsInterpolatedWalkDistance` returns
  `-(walkDist + (walkDist - walkDistO) * partialTick)`;
  `getInterpolatedBob` linearly interpolates `bobO` to `bob`.
- `GameRenderer.renderLevel`: projection multiplication order is
  `bobHurt`, `bobView`, then spinning distortion. `bobView` translates, rotates
  Z, then rotates X using the exact sine/cosine formulas.

## Frozen ownership and API

- `bbb-world` remains the owner of effective local sprint eligibility and the
  existing player cloak/avatar tick state. It exposes one copy-only camera
  view-bob projection; it does not add another animation clock.
- `bbb-native::camera_pose` owns the retained `Camera` eye-height tick state and
  active camera-entity selection. A copy of this state travels with the
  immutable simulation publication so render-time partial ticks cannot read a
  newer mutable value.
- `bbb-native::camera_fov` receives the already revalidated effective sprint
  bit from `ClientInputState`; it does not infer sprinting from velocity or the
  swimming pose.
- `bbb-renderer` owns the view-bob matrix and applies it in the shared camera
  projection path. `CameraViewBobEffect` is a pure value type. The existing
  hurt and spinning APIs keep their behavior and order.
- Startup configuration gains only the Vanilla view-bobbing boolean
  (`--client-view-bobbing`, default true); no in-game configuration UI is
  introduced.

## Explicit non-goals

- Fluid travel, sprint eligibility, swimming pose selection, player swim-arm
  animation, and cape state are already owned and are not reimplemented.
- This slice does not add camera roll, free-look, cinematic smoothing,
  accessibility UI, or change third-person collision clipping.
- Server-authoritative remote movement and packet formats do not change.
- Input raycasts keep their existing canonical pose query; this contract owns
  render-camera pose smoothing, matching the current native presentation
  boundary.

## Serial stages, tests, and checkpoint

The fully gated baseline is `cf1aba723`.

1. **Sprint FOV and retained eye height.** Thread the effective sprint bit into
   the FOV target, add the Vanilla `Camera.tick` eye-height state, publish it
   immutably, and use it before first/third-person camera placement.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world camera_fov`,
   `scripts/cargo-dev.sh test -p bbb-native camera_eye_height`, and
   `scripts/cargo-dev.sh test -p bbb-native camera_fov`.
2. **View bob and closeout.** Expose the existing avatar walk/bob pair, add the
   renderer value/matrix in exact order, connect the startup option and frame
   setter, prove swimming decay, close the ledger and archive this contract.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world camera_view_bob`,
   `scripts/cargo-dev.sh test -p bbb-native camera_view_bob`,
   `scripts/cargo-dev.sh test -p bbb-renderer camera_view_bob`, and
   `scripts/cargo-dev.sh test -p bbb-native startup`.

Both stages are buildable safe stops. Run the per-commit checks before each
commit and the full gate after stage 2. Failures are fixed forward; camera state
never lands without its immutable publication path.
