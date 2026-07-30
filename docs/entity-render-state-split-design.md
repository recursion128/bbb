# Entity Render-State Split Engineering Contract

Status: active.

## Goal and completion

Close the active Vanilla 26.1 fine-grained/coalesced GPU render-state gap for
the complete repo-native entity/block-entity render-type surface. Remove every
remaining semantic state merge among the 29 typed
`EntityModelLayerRenderType` values without changing the CPU submission graph,
texture residency, model geometry, or world-to-render extraction boundary.

Completion requires:

- `entitySolid` to preserve low-alpha texels while `entityCutoutCull` keeps the
  `0.1` alpha cutoff;
- End Crystal beam, Breeze wind, and End Gateway beacon-beam geometry to select
  their exact Vanilla pipelines instead of one shared translucent scroll
  pipeline;
- the End Gateway beam's opaque core and translucent glow to be separate typed
  submissions and draw ranges;
- all 29 typed render states to have a mechanically exhaustive bucket/pipeline
  disposition;
- the existing dynamic `LightTexture`, darkness-adjusted brightness gamma, and
  diffuse-lighting paths to be reverse-audited against Vanilla and guarded as
  closed rather than reimplemented;
- focused CPU/GPU semantic tests, removal of the ledger item, archival of this
  contract, a concise milestone entry, and a passing full gate at the final
  committed head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/RenderPipelines.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/rendertype/RenderTypes.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/Lightmap.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/blockentity/BeaconRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/blockentity/TheEndGatewayRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/EndCrystalRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/layers/BreezeWindLayer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/assets/minecraft/shaders/core/entity.vsh`; and
- `/home/zgy/Work/mc-code/sources/26.1/assets/minecraft/shaders/core/entity.fsh`.

`ENTITY_SOLID` uses the entity shader, lightmap and overlay with back-face
culling, opaque replacement, depth writes, and ordinary one-sided diffuse
lighting. It has no `ALPHA_CUTOUT`. `ENTITY_CUTOUT_CULL` differs by adding the
`0.1` alpha cutoff; both keep one-sided diffuse lighting.

`END_CRYSTAL_BEAM` is opaque replacement with depth writes, no culling,
`ALPHA_CUTOUT 0.1`, `NO_OVERLAY`, and ordinary cardinal lighting. Its animated
UVs are baked by `EndCrystalRenderer.submitCrystalBeams`; it does not use the
texture-matrix scroll define.

`BREEZE_WIND` uses translucent blending, depth writes, no culling,
`ALPHA_CUTOUT 0.1`, `APPLY_TEXTURE_MATRIX`, `NO_OVERLAY`, and
`NO_CARDINAL_LIGHTING`, while retaining the entity lightmap.

`BeaconRenderer.submitBeaconBeam` submits two render types in order. The inner
core uses `BEACON_BEAM_OPAQUE`: opaque replacement, depth writes, no culling,
fog, and no entity lightmap/overlay/diffuse stage. The outer glow uses
`BEACON_BEAM_TRANSLUCENT`: translucent blending, `LessEqual`, no depth writes,
and the same shader inputs. The glow alpha is exactly `32/255`.

`LightTexture` is a dynamic 16x16 RGBA8 texture. Its lightmap shader applies
ambient and night-vision colors, sky/block factors, boss darkening, darkness
subtraction, clamping, `notGamma`, and the user brightness factor in that
order. Vanilla entity/item diffuse lighting uses the current `Lighting.Entry`
two-direction mix; no-cull entity pipelines with `PER_FACE_LIGHTING` invert
the normal on back faces, while cull-on pipelines use the ordinary normal.

## Ownership and frozen interfaces

- `EntityModelLayerRenderType` remains the single typed render-state identity.
  It gains distinct End Gateway opaque/translucent beam values; the old
  combined value is removed. Its exhaustive `ALL`, bucket, blending, culling,
  outline, crumbling, and Vanilla-name methods remain the authority.
- `EntityModelTexturedMeshes` keeps compatible vertex formats but separates
  semantic draw ownership: a dedicated static `entitySolid` mesh, a dedicated
  End Crystal beam mesh/range, a Breeze wind mesh/range, and distinct End
  Gateway opaque/translucent beam meshes/ranges. No generic per-draw boolean
  state override is introduced.
- `bbb-renderer::Renderer` owns dedicated pipelines selected solely by typed
  render identity. Opaque End Crystal/End Gateway beam ranges draw in the main
  solid feature phase. Breeze and End Gateway glow remain in the sorted main
  translucent feature plan.
- The existing atlas rect and scroll vertex data remain reusable. Specialized
  shaders may ignore fields that Vanilla does not consume; no second texture
  atlas, dynamic residency owner, or world state is added.
- `LightmapEnvironment`, `LightmapGpu`, `CameraUniform`, item-model shaders,
  and terrain cardinal-lighting owners remain unchanged unless the reverse
  audit finds a concrete formula or ordering mismatch.

## Bounds and fallback rules

1. Missing atlas residency continues to omit only the affected typed
   submission through the existing bounded lookup.
2. Submission `order`, `submit_sequence`, transform, tint, packed light,
   overlay, outline color, camera-distance sort key, and stable insertion tie
   break remain unchanged except that one End Gateway submission becomes two
   consecutive typed submissions.
3. Opaque beam ranges do not enter the translucent sort plan. Breeze wind and
   End Gateway glow retain far-to-near sorting by the existing collector key.
4. End Gateway inner geometry precedes outer geometry and uses the same
   animation time, height, scale, color, texture, and atlas residency.
5. Dynamic player skin/profile atlases preserve their existing supported
   render-type set. Unsupported combinations remain explicit rather than
   falling into another state.
6. Every typed render state must be covered by the exhaustive disposition
   test; adding a future enum value without an exact bucket/pipeline decision
   fails compilation or tests.

## Non-goals

- This engineering slice does not change packet/world state, entity model
  geometry, animation, texture paths, pack reload, target-composite order,
  particle/terrain pipelines, fog formulas, standalone sampler policy, or mip
  generation.
- It does not introduce a generic runtime material graph, shader
  specialization cache, renderer/pack dependency, command-line graphics
  option, or in-game configuration UI.
- It does not alter currently exact render states merely to give every typed
  value a unique GPU object; compatible states may share implementation helpers
  when their Vanilla semantics are identical.

## Serial stages and verification

1. **Opaque entity state.** Split `entitySolid` from `entityCutoutCull`, add
   exact shader/pipeline and static residency ownership, and prove low-alpha
   solid versus discarded cutout behavior plus the exhaustive typed
   disposition. Run
   `scripts/cargo-dev.sh test -p bbb-renderer entity_model_solid`.
2. **Beam and wind state.** Split End Crystal, Breeze, and End Gateway CPU
   ranges and GPU pipelines; split End Gateway core/glow submissions and
   solid/translucent scheduling. Prove blend/cull/depth/lightmap/overlay/
   diffuse/texture-matrix state and deterministic geometry ordering. Run
   `scripts/cargo-dev.sh test -p bbb-renderer beam` and
   `scripts/cargo-dev.sh test -p bbb-renderer breeze`.
3. **Reverse audit and closeout.** Pin the already-present `LightTexture`
   formula/order, darkness/brightness projection, `Lighting.Entry`, item,
   terrain, cull-on and no-cull diffuse owners with focused structural and
   semantic tests. Run `scripts/cargo-dev.sh test -p bbb-renderer lightmap`
   and `scripts/cargo-dev.sh test -p bbb-renderer diffuse`; remove the ledger
   item, archive this contract, append the milestone index, run the standard
   per-commit checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`d508b0c44727b09101292b8e62d4f81c59b20e0b`. Because this changes central GPU
pipeline and pass dispatch, the final full gate is the engineering checkpoint.
No playable gate is required because protocol, world mutation, input, audio,
and control-command paths do not change.

## Failure localization and safe stops

Opaque-state failures compare typed mesh ownership and alpha-cutout shader
state. Beam/wind failures compare typed CPU submission/range ownership before
pipeline descriptor state and pass scheduling. Lightmap/diffuse audit failures
compare the existing uniform projection and shader formula without broadening
the implementation.

The clean contract commit and each buildable implementation stage are safe
stops. Focused or gate failures are fixed forward before another slice;
history is not rewritten.
