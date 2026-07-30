# Entity-Attached Block Outline Engineering Contract

Status: completed on 2026-07-29.

## Goal and completion

Close the retained Vanilla 26.1 `BlockModelRenderState.submitOnlyOutline` gap
for entity-attached block models. Invisible glowing snow-golem pumpkins and
mooshroom mushrooms must keep their blocks-atlas geometry out of the ordinary
block-feature lanes, render their texture alpha mask in the entity-outline
target with the entity's exact outline color, and activate the existing entity
outline post chain even when no textured entity outline mesh exists.

Completion requires native extraction/residency tests, renderer
phase/target/shader/pipeline/pass-order tests, removal of the active ledger
item, archival of this contract, a concise milestone index entry, and a
passing full gate at the final head.

## Authoritative behavior

- `SnowGolemHeadLayer.submit` and `MushroomCowMushroomLayer.submit` retain their
  block attachments when the entity is both invisible and glowing. That branch
  calls `BlockModelRenderState.submitOnlyOutline` with the normal posed block
  transform, packed light, overlay, and `state.outlineColor`; it does not call
  the ordinary `submit` path.
- `BlockModelRenderState.submitOnlyOutline` submits the existing model parts
  through `RenderTypes.outline(TextureAtlas.LOCATION_BLOCKS)`.
- `RenderTypes.outline(texture)` selects `OUTLINE_NO_CULL`, routes to
  `OUTLINE_TARGET`, and marks the render type as an outline. The pipeline uses
  `LessEqual`, writes depth, disables culling, and has no color-target blend
  override.
- `OutlineBufferSource.EntityOutlineGenerator` writes the selected outline
  color at `addVertex` and ignores the source quad's later color, light, overlay,
  and normal setters. The outline shader samples texture alpha only, discards
  exactly zero alpha, and emits the outline RGB with opaque alpha. It applies
  no block tint, lightmap, overlay, cardinal lighting, or fog.
- `LevelRenderer.addMainPass` clears the entity-outline color and depth targets
  before feature rendering when glowing entities exist. The entity-outline post
  chain runs after feature targets and before clouds/transparency composition.

Authoritative local sources are
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/layers/SnowGolemHeadLayer.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/layers/MushroomCowMushroomLayer.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/block/BlockModelRenderState.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/feature/BlockFeatureRenderer.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/OutlineBufferSource.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/rendertype/RenderTypes.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/RenderPipelines.java`,
and
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/LevelRenderer.java`.

## Ownership and frozen interfaces

- `bbb-native::item_models::dropped` remains the owner of entity-attached block
  discovery and blocks-atlas quad resolution. `EntityBlockAttachment` gains the
  exact `outline_color`; the existing `outline_only` discriminator remains the
  branch contract.
- `entity_block_models` continues to reuse ordinary resident block geometry
  owners. An outline-only attachment emits no `BlockFeature` instance. Instead,
  each non-empty solid/translucent geometry class emits one instance with
  `ItemModelDrawTarget::EntityOutline`, `ItemModelDrawPhase::Outline`, the
  unchanged posed transform, and the ARGB outline color converted to normalized
  RGBA tint. Light and overlay remain carried but are ignored by the outline
  shader.
- `bbb-renderer::item_models::residency` adds the explicit phase and target.
  Neither becomes part of `ItemModelAssetKey`, so one static geometry owner can
  serve ordinary and outline submissions without duplicate vertex/index
  residency.
- The renderer gains one blocks-atlas resident outline pipeline with the
  existing resident vertex/instance layouts. It samples the terrain bind group,
  has no lightmap bind group, uses replacement output, no culling, depth write,
  and `LessEqual`.
- `entity_outline_target_pass` treats resident outline draws as target content,
  clears once, then renders existing textured entity outline meshes and resident
  block outlines into the same attachments. `entity_outline_post_chain` uses
  the same combined-content predicate.

## Bounds and fallback rules

- Existing entity visibility, attachment count, resource residency owner/byte
  caps, and per-frame instance bounds remain authoritative; this slice adds no
  queue or persistent world state.
- Missing/empty blocks-atlas model geometry drops the attachment exactly as the
  ordinary path does. A zero outline color cannot produce an outline instance.
- Solid and translucent source geometry are both retained as alpha-mask input;
  the outline render state is identical for both and never falls back to an
  ordinary block-feature phase.
- If no textured entity outline mesh and no resident block outline draw exists,
  the target and post-chain passes remain skipped.

## Non-goals

- This slice does not add the outline copy produced when ordinary
  `BlockModelRenderState.submit(..., outlineColor)` uses a render type that
  affects outlines. That broader normal-surface-plus-outline path must be
  audited and tracked separately from `submitOnlyOutline`.
- It does not change attachment visibility gates, entity transforms, block
  model resolution/tints, moving-block rendering, or ordinary solid/translucent
  block-feature ordering.
- It does not redesign the entity-outline post effect, target sizing/depth,
  texture sampler/mipmap policy, or textured entity outline meshes.
- It does not add an in-game fixture; deterministic extraction, residency,
  shader-state, and render-order tests are the acceptance surface.

## Serial stages and verification

1. **Native outline residency lane.** Retain outline-only attachment geometry,
   carry exact color, emit only the explicit outline target/phase, and preserve
   ordinary block-feature behavior. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native entity_block` and
   `scripts/cargo-dev.sh test -p bbb-renderer item_model`.
2. **GPU target and post-chain integration.** Add the blocks-atlas resident
   outline shader/pipeline, draw it in the shared target, and include it in the
   target/post activation predicate. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-renderer entity_outline` and
   `scripts/cargo-dev.sh test -p bbb-renderer resident_item_model`.
3. **Closeout.** Re-run all focused families, update/archive the ledger and
   contract, append the milestone index, perform the standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`08b95a5a1f040f750aa6567ca344c2b0fdb95414`. Stage 1 is the first safe stopping
point; it must remain buildable even before the renderer starts drawing the new
target.
