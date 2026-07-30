# Banner Pattern Pipeline Engineering Contract

Status: active from 2026-07-30.

## Goal and completion

Close the active Vanilla 26.1 `bannerPattern` render-state gap by giving banner
pattern layers their own typed submission and GPU pipeline instead of folding
them into generic `entityTranslucent`.

Completion requires the dedicated pipeline to preserve translucent blending,
`LessEqual` depth testing, back-face culling, lightmap/fog and sorted upload
order while disabling depth writes, overlay, alpha cutout, outline copying and
crumbling. Static built-in patterns and synchronized runtime-pattern atlas
entries must use the same typed state in world rendering and isolated GUI
banner previews. Completion also requires focused CPU submission tests,
pipeline descriptor/shader guards, deterministic offscreen GUI evidence,
removal of the ledger item, archival of this contract, a milestone index entry,
and a passing full gate at the final head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/RenderPipelines.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/rendertype/RenderTypes.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/blockentity/BannerRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/assets/minecraft/shaders/core/entity.vsh`; and
- `/home/zgy/Work/mc-code/sources/26.1/assets/minecraft/shaders/core/entity.fsh`.

`RenderPipelines.BANNER_PATTERN` inherits the entity shader and adds
`NO_OVERLAY`, translucent blending, and
`DepthStencilState(LESS_THAN_OR_EQUAL, false)`. It does not add
`ALPHA_CUTOUT`, `PER_FACE_LIGHTING`, or `withCull(false)`, so it keeps
back-face culling and ordinary cardinal entity lighting. `RenderTypes`
provides the texture and lightmap, marks the type sorted-on-upload, and does not
mark it as affecting outline or crumbling.

`BannerRenderer.submitPatterns` submits the base-color mask first and then at
most 16 stored pattern layers. Every layer reuses identical posed flag geometry
and the dedicated render type. Equal-depth `LessEqual` therefore preserves
layer visibility and submission order, while the no-write state prevents the
translucent masks from becoming depth occluders for later geometry.

## Ownership and frozen interfaces

- `EntityModelLayerRenderType` gains the exact `BannerPattern` identity.
  `banner_textured_layer_passes` assigns it to the base-color mask and every
  stored layer. No boolean depth override is attached to generic translucent
  submissions.
- The CPU mesh owner preserves static and runtime-atlas banner patterns as
  distinct typed sorted ranges while reusing the compatible translucent vertex
  buffers. Existing submission order, sort keys, tint, transform, light, UV
  and atlas selection remain unchanged.
- `bbb-renderer::Renderer` owns one dedicated banner-pattern pipeline and the
  existing static/dynamic atlas residency. World translucent range dispatch
  selects it solely from the typed render identity.
- Isolated HUD entity previews gain a `BannerPattern` PIP pipeline lane and
  preserve the same cutout/glint/translucent ordering. Loom thumbnails that are
  ordinary 2D atlas quads remain on the HUD pipeline; only the flag-model
  result preview uses the entity banner pipeline.
- The dedicated WGSL path samples the entity atlas and lightmap, applies
  cardinal entity lighting and fog, and multiplies the pattern tint. It does
  not read overlay coordinates and does not discard low-alpha texels.

## Bounds and fallback rules

1. Missing static or runtime atlas residency continues to omit only the
   affected draw through the existing bounded atlas lookup. Runtime patterns
   do not fall back to a different synchronized holder or handwritten ordinal.
2. The base mask and all present stored layers share the dedicated state; no
   layer count, tint, animation, geometry, or texture-identity behavior changes.
3. World sorted draws remain ordered by explicit order, camera distance, then
   insertion index. Equal sort keys retain Vanilla submission order.
4. The pipeline keeps `LessEqual`; changing it to `Less` would erase equal-depth
   layers and is outside the contract. Only depth writes are disabled.
5. Ordinary `entityTranslucent`, armor translucent, emissive, item-target,
   outline, HUD 2D, and other banner base/flag pipelines retain their current
   state.

## Non-goals

- This slice does not change banner packet/state decode, registry holder
  resolution, atlas generation, block-entity animation, model geometry,
  crumbling geometry, item rendering, or loom interaction.
- It does not generalize all coalesced GPU states, samplers, mip generation,
  translucent sorting, or render-graph ownership.
- It does not add a runtime pipeline configuration surface or in-game
  configuration UI.

## Serial stages and verification

1. **Typed submission and residency.** Add the `BannerPattern` render identity,
   static/runtime sorted ranges, exact no-overlay/crumbling/outline metadata,
   and world/PIP dispatch. Update banner submission and PIP ordering tests.
2. **GPU state and proof.** Add the dedicated shader/pipeline with translucent
   blend, back cull, `LessEqual`, no depth write, no overlay and no alpha
   cutoff. Lock descriptor constants and shader structure, and extend the
   existing loom result offscreen proof to consume the dedicated PIP lane.
3. **Closeout.** Run `scripts/cargo-dev.sh test -p bbb-renderer banner`,
   `scripts/docs-lint.sh`, formatting and diff checks; remove the ledger item,
   archive this contract, append the milestone index, review the integrated
   diff, commit clean, and run the full gate.

The pre-start full gate passed at
`51aa7c50922c96dfdcafbf5bbb8a73f29d431dc3`. Because this changes a GPU
pipeline and central render dispatch, the final full gate is the engineering
checkpoint. No playable gate is required because protocol, world mutation,
input, and control-command paths do not change.

## Failure localization and safe stops

CPU failures compare layer render identity, static/runtime mesh ownership,
submission order and atlas selection before GPU code. Pipeline failures compare
shader inputs, blend/cull/depth constants and world/PIP dispatch before
offscreen pixels.

The clean contract commit is the first safe stop. Any focused or gate failure
is fixed forward before another slice; history is not rewritten.
