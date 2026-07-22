# Block-destruction packet coverage closure

## Goal and completion contract

Close the remaining runtime-side-effect coverage cell for
`ClientboundBlockDestructionPacket`. The accepted destroy progress must keep
one canonical highest stage per block position and render `destroy_stage_0..9`
over the exact selected block-model quads and over each supported block-entity
model submission that Vanilla marks as crumbling-aware.

Completion means arbitrary baked block quads no longer degrade to a full cube,
empty/non-model shapes emit no crack geometry, the model's posed normals drive
the exact `SheetedDecalTextureGenerator` projection, block-entity model
submissions consume the same merged server/local stage as terrain cracks,
Vanilla's render-type and per-renderer exclusions are preserved, all five
packet coverage dimensions are `complete`, the two finished crack TODOs are
removed or narrowed to their still-open culling work, and the full plus
playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-world` remains the sole owner of server destruction ids, positions,
  stages, expiry, and local-player destroy prediction.
- `bbb-native` owns one deterministic server/local highest-stage projection.
  It publishes terrain overlays and annotates block-entity model instances
  from that same projection; it does not create a second destruction state.
- `bbb-renderer` owns exact baked-quad decal geometry, the block-entity
  crumbling render-state value, render-type eligibility, posed-model geometry
  duplication, persistent frame-buffer upload, and the existing crumbling
  pipeline/pass.

This slice does not change destroy speed, `STOP_DESTROY_BLOCK` sequencing,
block `attack` callbacks, hit particles, authoritative block removal, or
server collision results. It does not add missing block-entity renderers.
Per-block-entity distance/frustum culling remains a separate visibility and
performance slice; it must not be coupled to whether an existing model receives
the correct destroy overlay. Configuration remains command-line only.

## Vanilla and integration contract

`ClientPacketListener.handleBlockDestruction` delegates to
`ClientLevel.destroyBlockProgress`, and `LevelRenderer.destroyBlockProgress`
indexes progress both by breaker id and block position. At extraction, the
last entry of the position's sorted set supplies the visible stage. The
existing bbb projection retains the same highest-stage rule while merging the
local player's predicted stage.

`LevelRenderer.submitBlockDestroyAnimation` only submits states whose render
shape is `MODEL`, selects the position-seeded `BlockStateModel`, and passes its
actual parts and quads to `submitBreakingBlockModel`. `BlockFeatureRenderer`
then writes those quads through `SheetedDecalTextureGenerator` with no tint.
Therefore `TerrainRenderShape::Quads` must emit exactly its selected corners
and normals in existing model order; an empty quad list emits nothing. For each
vertex, the nearest cardinal direction is selected from the posed normal. The
position is projected using Vanilla's `rotateY(PI)`, `rotateX(-PI/2)`, then
`direction.getRotation()` sequence at texture scale `1`, before mapping into
the selected destroy-stage atlas rectangle. The existing crumbling blend,
alpha cutoff, back-face cull, `LESS_EQUAL` depth compare, disabled depth write,
and depth bias remain authoritative.

During visible block-entity extraction, `LevelRenderer` attaches a
`ModelFeatureRenderer.CrumblingOverlay` containing the same highest stage and
a camera pose translated to the block position. `ModelFeatureRenderer`
duplicates a submitted posed model only when both an overlay was passed and
its `RenderType.affectsCrumbling()` flag is set. In bbb the corresponding
instance value carries the block origin and resolved destroy-stage UV. The
textured submission path converts only that submission's already-posed
vertices and indices into the shared crumbling vertex format. UV projection
uses `world_position - block_origin`, while the already-posed normal selects
the nearest direction; colour is forced white just like the Vanilla wrapper.

Native annotates the currently implemented chest, standing/hanging sign, bed,
bell, shulker box, banner, enchanting-table book, lectern book, skull, and the
conduit's shell/cage instances. Conduit wind and eye submissions receive no
overlay in `ConduitRenderer`. Decorated-pot submissions explicitly pass
`null`; end portal/gateway custom geometry and spawner-displayed entities do
not consume the base block-entity state's overlay. The render-type eligibility
table follows `RenderTypes.java`: entity solid/cutout/cull/z-offset,
translucent variants, dissolve, and armor types affect crumbling, while banner
pattern, eyes, scroll/glint, portal, line, beam, outline-only, and depth-only
types do not. Thus banner base and flag geometry crack, but its pattern
re-submissions do not.

Authoritative local sources are
`net/minecraft/client/multiplayer/ClientPacketListener.java`,
`net/minecraft/client/multiplayer/ClientLevel.java`,
`net/minecraft/client/renderer/LevelRenderer.java`,
`net/minecraft/client/renderer/feature/BlockFeatureRenderer.java`,
`net/minecraft/client/renderer/feature/ModelFeatureRenderer.java`,
`com/mojang/blaze3d/vertex/SheetedDecalTextureGenerator.java`,
`net/minecraft/client/renderer/rendertype/RenderTypes.java`, and the relevant
classes under `net/minecraft/client/renderer/blockentity/` in the checked-in
Vanilla 26.1 source tree.

## Serial stages and verification

1. Replace the `TerrainRenderShape::Quads` cube fallback with exact corner,
   normal, nearest-direction, winding, and decal-UV emission; make an empty
   model emit nothing. Focused tests: `bbb-renderer` block-destroy tests for a
   rotated/non-axis-aligned quad, nearest-direction projection, stable order,
   and empty geometry.
2. Add the block-entity crumbling render-state value and exact
   `affectsCrumbling` classification, duplicate eligible posed textured
   submissions into a dedicated CPU mesh, upload it through persistent frame
   buffers, and draw it in the existing block-destroy pass. Focused tests:
   renderer instance defaults/builders, all render-type classifications,
   chest/banner/conduit/skull geometry selection, posed decal UVs, and
   render-pass/frame-buffer source assertions. This core renderer milestone
   gets a full gate.
3. Factor the native highest-stage projection, annotate only the Vanilla
   block-entity consumers above, and prove server/local merging, stage removal,
   per-kind exclusions, and the shared stage UV contract. Focused tests:
   `bbb-native` runtime extraction regressions plus the renderer tests.
4. Mark the packet runtime cell complete, remove the finished full-model crack
   TODO, narrow the block-entity TODO to distance/frustum culling, append the
   milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `44d91a042` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Block-quad failures stop before the model-submission path;
renderer duplication/upload failures stop before native annotation; native
selection failures stop before coverage closeout. Failures are fixed forward
without rewriting history.
