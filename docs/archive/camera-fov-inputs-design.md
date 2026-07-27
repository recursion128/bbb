# Camera FOV Input Closure

Status: completed on 2026-07-26.

## Goal and completion standard

Close the remaining camera-FOV input gap as one native/renderer engineering
slice. Startup configuration, synchronized player state, camera-entity state,
fluid probes, render interpolation, CPU visibility, locator projection, and
the first-person GPU pass must agree with Vanilla 26.1.

Completion requires:

- `--client-fov` accepts the Vanilla integer range 30 through 110 and defaults
  to 70;
- `--client-fov-effect-scale` accepts finite values from 0 through 1 and
  defaults to 1;
- the existing tick-smoothed player modifier consumes the configured effect
  scale while retaining synchronized `Abilities.walkingSpeed` as its ratio
  denominator and the movement-speed attribute as its numerator;
- death and water/lava camera modifiers are applied after the interpolated
  world modifier;
- render, culling, and first-person HUD angles remain distinct:
  world FOV uses the configured base and smoothed modifier, culling uses
  `max(world FOV, configured base)`, and HUD FOV starts at 70 without the
  player modifier;
- both world and HUD FOV receive the same death/fluid modifier, while locator
  projection consumes world FOV and CPU entity visibility consumes culling
  FOV;
- the unsupported-feature todo is removed, this contract is archived, the
  milestone index is updated, and the final clean HEAD passes the full gate
  and P0 audit.

## Scope and non-goals

Owned paths are the native startup configuration, camera-FOV tick/frame owner,
simulation publication, camera-fluid probes, render extraction, CPU entity
frustum input, renderer camera uniforms and texture bindings used by the
first-person pass, focused tests, and closeout documentation.

This slice does not add an in-game options UI or an options persistence format;
repo policy keeps configuration at command-line startup. It does not add
panoramic rendering, third-person cameras, missing lava fog presentation,
powder-snow overlays, view bobbing, portal FOV inputs, or other survival
physics. It does not change the synchronized movement-speed attribute or
`Abilities.walkingSpeed` ownership. The CLI effect-scale value is the effective
value read by `Options.fovEffectScale().get()`, not the options-screen slider's
square-root display mapping.

## Vanilla 26.1 evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `Options.fov` is an integer `OptionInstance` with range 30 through 110 and
  default 70. `Options.fovEffectScale` has effective range 0 through 1 and
  default 1.
- `AbstractClientPlayer.getFieldOfViewModifier` applies flying, divides the
  movement-speed attribute by synchronized `Abilities.walkingSpeed`, applies
  bow/scoping state, and finally interpolates from 1 by the configured effect
  scale.
- `Camera.tickFov` samples that target once per client tick, eases halfway from
  the current modifier, and clamps the modifier to 0.1 through 1.5. A
  non-player camera entity targets 1.
- `Camera.calculateFov` multiplies the configured base by the interpolated
  modifier. `calculateHudFov` instead starts at 70. Both then call
  `modifyFovBasedOnDeathOrFluid`.
- Death divides the angle by
  `(1 - 500 / (min(deathTime + partialTick, 20) + 500)) * 2 + 1`.
  Water or lava multiplies it by
  `lerp(fovEffectScale, 1, 0.85714287)`.
- `Camera.getFluidInCamera` tests water at the camera position, then tests lava
  at the forward point and four corners of the 0.05-near plane constructed
  from the configured base FOV.
- `Camera.createProjectionMatrixForCulling` uses
  `max(calculated world FOV, configured base FOV)` and does not apply camera
  hurt, view bob, or spinning transforms.
- `GameRenderer.renderLevel` uses the calculated world projection, then
  switches to a separate perspective projection with `cameraState.hudFov`
  before `renderItemInHand`.

## Boundaries and frozen integration API

- A new `bbb-native::camera_fov` module owns `CameraFovTickState`,
  `CameraFovFrame`, the Vanilla formula constants, and camera fluid sampling.
  The state is still a small `Copy` simulation value published with each
  immutable generation. Its constructor accepts validated base FOV and effect
  scale; `Default` retains Vanilla 70/1 behavior for fixtures.
- `advance_for_world` remains immediately after client input/key handling and
  before render publication. It delegates the player target to
  `WorldStore::local_player_camera_fov_modifier(first_person, effect_scale)`.
- Render extraction computes one `CameraFovFrame` after the interpolated camera
  pose and camera living-state projection are available. The frame contains
  `world_degrees`, `culling_degrees`, and `hud_degrees`; no consumer repeats
  death/fluid formulas.
- The existing locator projection receives `world_degrees`.
  `EntityRenderVisibilityContext` receives `culling_degrees`.
  `RendererFrame` carries only the GPU-relevant world and HUD angles and
  applies them through one renderer setter before the camera pose.
- `bbb-renderer::Renderer` owns a world camera buffer and a new `hud3d` camera
  buffer. Texture resources shared by world, GUI, and first-person rendering
  retain one texture/sampler but expose bindings for the applicable camera
  buffer. The first-person pass selects only `hud3d` bindings; world and GUI
  passes retain their current bindings.
- The world uniform keeps current hurt/spinning projection inputs. The `hud3d`
  uniform uses HUD FOV and the hurt transform but excludes the world-only
  spinning projection, matching the Vanilla hand projection boundary.
- FOV degree sanitization must admit every configured/modifier result produced
  by the Vanilla ranges; the current 7-through-105 renderer clamp is removed.
  Non-finite out-of-contract values fall back to 70 without widening startup
  input.
- Missing camera pose, unloaded fluid blocks, missing atlases, and absent
  first-person geometry retain their existing safe fallback/no-draw behavior.

## Serial stages, tests, and safe stops

1. **Native FOV owner and startup inputs.** Extract the existing tick owner,
   add validated CLI inputs, compute world/culling/HUD frames including
   camera-entity death and Vanilla water/lava probes, and route world versus
   culling consumers.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native camera_fov` and
   `scripts/cargo-dev.sh test -p bbb-native fov_startup`.
2. **Renderer projection separation.** Add the persistent `hud3d` camera
   buffer/bindings, route every first-person atlas and resident item phase to
   it, widen valid perspective FOV handling, and add projection/binding
   regression tests.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-renderer camera_fov`.
3. **Integration and closeout.** Lock renderer-frame setter ordering, remove
   the ledger todo, archive this contract, append the engineering milestone,
   and run the per-commit checks.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-native camera_fov`.

Each stage ends in a buildable normal commit. The final closeout runs
`scripts/cargo-dev.sh gate` followed by `scripts/audit-p0.sh`. Failures are
repaired forward from the preceding clean commit. World/HUD values travel
together, so no compatibility mode or partially switched projection state is
allowed.

## Hardest-ready decision

This is the hardest ready candidate after F1 presentation parity. It crosses
startup configuration, simulation tick state, immutable publication, camera
entity queries, fluid geometry, CPU visibility, locator math, GPU uniform
ownership, and every first-person texture binding, but Vanilla fixes all
formulas and ordering and the existing camera owner makes the integration
contract bounded. Remaining survival physics has an open-ended collision and
travel scope. The Tutorial-backed inventory side effect still lacks a
freezeable complete contract for step transitions, item tags, Toast
presentation, startup/persistence state, and non-packet inputs.
