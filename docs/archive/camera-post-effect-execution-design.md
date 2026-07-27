# Spectator Camera Post-Effect Execution

Status: completed on 2026-07-26.

## Goal and completion standard

Execute the three built-in Vanilla 26.1 camera-entity post chains that bbb
already selects and mirrors in the debug overlay:

- `minecraft:creeper`;
- `minecraft:spider`; and
- `minecraft:invert`.

The selection remains generated from
`GameRenderer.checkEntityPostEffect` plus the entity class hierarchy. This
engineering slice adds the independent `effectActive` lifecycle, the default
F4 toggle, persistent renderer targets/pipelines, exact built-in pass graphs,
resize handling, counters, and deterministic GPU readback evidence.

Completion requires:

- a newly selected supported camera effect starts active, repeated extraction
  of the same selection preserves an F4-disabled state, every applied
  `SetCamera` selection refresh reenables its supported effect, and clearing
  the selection clears active state;
- ordinary F4 press edges toggle active state once, while held-key repeats and
  the consumed F3+F4 game-mode-switcher chord do not double-toggle it;
- selection refresh follows the already established network-before-input
  simulation order and survives skipped/coalesced render publications as one
  final immutable state rather than a render-thread toggle queue;
- the renderer executes Creeper's two passes, Spider's ten passes, and
  Enderman invert's two passes after the world and first-person hand have
  reached the frame target but before vignette, GUI PIP composition, ordinary
  HUD, and screenshot readback;
- all intermediate targets are persistent, full physical-frame sized, and
  recreated on a nonzero resize; no per-frame GPU buffer, texture, bind-group,
  or pipeline is created;
- focused native and renderer tests cover lifecycle/order, shader constants,
  pass counts, disabled fallback, resize, and offscreen pixel boundaries;
- the remaining perspective lifecycle is stated as its own positive TODO, this
  contract is archived, the milestone index is updated, and final clean HEAD
  passes the full gate and P0 audit.

## Scope and non-goals

Owned paths are native input/runtime publication, the existing renderer-frame
post-effect field, renderer counters, a new renderer-owned camera post-effect
module, frame orchestration, resize, offscreen readback tests, and closeout
documentation.

This slice does not implement F5 or third-person front/back cameras. Therefore
it cannot yet execute Vanilla's `CameraType.isFirstPerson` transition that
clears the chain on leaving first person and reselects it on return; that
remains a narrower explicit TODO rather than a simulated perspective flag.

It does not add a general selected-pack `post_effect/*.json` graph loader or a
runtime GLSL-to-WGSL shader compiler. The three official built-in graphs are
translated to fixed renderer pipelines, as existing terrain/HUD/effect shaders
are. General resource-pack post graph/shader overrides remain resource
presentation work. It does not add unrelated screen-effect overlays, portal
inputs, screen blur changes, entity-outline changes, or a new configuration UI.

Missing camera entities, unsupported effect ids, inactive effects, zero-sized
resize requests, unavailable optional offscreen adapters, and absent scene/HUD
geometry retain their existing no-effect/no-draw/skip behavior. An unknown id
never falls back to a different built-in effect.

## Vanilla 26.1 evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `GameRenderer.checkEntityPostEffect` selects Creeper, Spider (including
  subclasses through Java pattern matching), or EnderMan; every selection calls
  `setPostEffect`, which stores the id and sets `effectActive = true`.
  The default/null branch calls `clearPostEffect`, clearing both fields.
- `GameRenderer.togglePostEffect` flips only `effectActive`. `Minecraft`
  consumes `keyToggleSpectatorShaderEffects` (default F4) independently from
  selection.
- `ClientPacketListener.handleSetCamera` calls
  `Minecraft.setCameraEntity` for every resolved packet, including a repeated
  selection; `setCameraEntity` always calls `checkEntityPostEffect`.
- `Minecraft.handleKeybinds` processes a perspective edge by passing either the
  current camera entity or null to `checkEntityPostEffect`. That behavior
  remains blocked on bbb's missing third-person camera owner.
- `GameRenderer.render` runs the selected post chain after `renderLevel`
  (world, hand, and screen effects) and entity outline processing, then renders
  GUI/HUD. bbb's equivalent insertion point is immediately after
  `first_person_item_pass` and before vignette/PIP/HUD presentation.
- `post_effect/creeper.json` runs `color_convolve` with matrices
  `(0,0,0)`, `(0.3,0.59,0.11)`, `(0,0,0)`, then `bits` with resolution
  16, mosaic size 4, and saturation 1.5.
- `post_effect/invert.json` runs `invert` with inverse amount 0.8, then an
  identity-color `blit`.
- `post_effect/spider.json` creates `large_blur`, `small_blur`, `temp`, and
  `swap`: horizontal/vertical linear box blur at radius 15, the same at radius
  7, five ordered `rotscale` + `spiderclip` passes with the exact JSON scale,
  offset, rotation, scissor, and vignette constants, then a blit modulated by
  `(1, 0.8, 0.8, 1)`.
- `box_blur.fsh` uses linear half-step sampling from
  `-radius + 0.5` through `radius` in steps of two, adds the last sample at
  half weight, and divides by `radius + 0.5`. The existing menu screen-blur
  pipelines already implement this kernel and will be reused.
- Non-bilinear post inputs use the graph's default nearest sampling. All
  declared targets without explicit dimensions inherit the main target size.

## Boundaries and frozen integration API

- `ClientInputState` owns the simulation-side active boolean plus a private
  last-selection token. The token combines client-level generation, applied
  `SetCamera` count, and the generated current effect id. A changed token sets
  active to `id.is_some()`; an unchanged token preserves the F4 result.
- `advance_simulation` synchronizes that token after the single network drain
  and before ordered input commands. The ordinary F4 press-edge branch toggles
  the boolean. `render_publication` carries only the final active boolean, not
  the private token or an edge queue.
- `current_post_effect_id_from_world` remains the sole native projection and
  must keep calling
  `bbb_world::vanilla_entity_camera_post_effect_id`; no resource-id or
  entity-type switch is added.
- `RendererFrame` carries the same current id used by the debug HUD and the
  paired active boolean. `apply_renderer_frame` commits both through one
  `Renderer::set_camera_post_effect(Option<&str>, bool)` call.
- `bbb-renderer::camera_post_effect` owns the effect-id enum, persistent source,
  temp/swap and Spider large/small targets, static uniforms/bind groups, and
  translated post pipelines. It reuses the renderer's existing horizontal and
  vertical screen-blur pipelines for the two Spider blur pairs.
- One frame copy captures the swapchain/offscreen target after the
  first-person pass. Internal passes ping-pong without source/destination
  aliasing; only the final pass writes back to the frame target.
- The thin `camera_post_effect_pass` step remains in `render.rs` so
  `render()` stays a pure ordered orchestrator and `FRAME_STEPS` continues to
  pin call/definition order. The module returns a draw count; frame accounting
  adds it to `post_effect_draw_calls`, total draw calls, and pipeline switches.
- Resize recreates only size-dependent post targets/bind groups. Pipelines,
  samplers, effect selection, and active state survive.

## Serial stages, tests, and safe stops

1. **Simulation lifecycle and frame handoff.** Add selection synchronization,
   F4 edge behavior, immutable active-state publication, atomic renderer
   setter, counters schema, and ordering/state tests.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native camera_post_effect` and
   `scripts/cargo-dev.sh test -p bbb-renderer camera_post_effect`.
2. **Persistent GPU graphs.** Add the semantic renderer module, persistent
   targets/uniforms/bind groups, exact Creeper/Spider/Invert graphs, resize and
   ordered frame step, plus shader and offscreen pixel tests.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-renderer camera_post_effect`.
3. **Integration and closeout.** Rewrite the ledger entry to only the genuine
   third-person/resource-pack remainder, archive this contract, append the
   engineering milestone, and run per-commit checks.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native camera_post_effect` and
   `scripts/cargo-dev.sh test -p bbb-renderer camera_post_effect`.

Each stage ends in a buildable normal commit. The final closeout runs
`scripts/cargo-dev.sh gate` followed by `scripts/audit-p0.sh`. Failures are
repaired forward from the preceding clean stage. The latest fully gated
baseline before this engineering slice is `773e2d024`.

## Hardest-ready decision

This is the hardest ready bounded candidate after camera-FOV closure. It crosses
network/input ordering, immutable simulation publication, render-frame
integration, full-surface copy/target ownership, six shader families, ten-pass
Spider ordering, resize, counters, and deterministic GPU readback. Vanilla
fixes every selected id, graph edge, sampler, uniform, and frame boundary.

The remaining survival movement/collision/travel family is still not one
freezeable semantic contract, and the Tutorial pickup partial lacks its full
step/tag/toast/startup-persistence boundary. Packet partials for ServerData and
Mount presentation are deferred; ContainerClick retains explicitly
server-authoritative component/result fallbacks. Creative-tab player preview
and narrow entity/audio presentation todos are ready but have less state,
ordering, and verification risk than this graph execution.
