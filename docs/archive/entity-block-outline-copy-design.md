# Ordinary Entity Block Outline-Copy Engineering Contract

Status: completed on 2026-07-30.

## Goal and completion

Close the active Vanilla 26.1 outline-copy gap for entity-attached block models
submitted through ordinary `BlockModelRenderState.submit(..., outlineColor)`.
Every extracted ordinary attachment with a nonzero entity outline color must
keep its normal block-feature submission and add an outline-target copy of the
same blocks-atlas geometry. This covers visible glowing snow-golem and
mooshroom blocks plus the iron-golem flower, enderman carried block, minecart
display block, primed/minecart TNT, and copper-golem antenna families.

Completion requires native family and residency tests, renderer residency and
shared-target/post-chain tests, deterministic GPU evidence that the ordinary
surface and its outline copy are both consumed, removal of the active ledger
item, archival of this contract, a concise milestone index entry, and a passing
full gate at the final head.

## Authoritative behavior

- `BlockModelRenderState.submit` records its cutout or translucent block model
  once with the supplied `outlineColor`. Both
  `Sheets.cutoutBlockSheet()` and `Sheets.translucentBlockSheet()` select render
  types marked `AFFECTS_OUTLINE`.
- `BlockFeatureRenderer.renderBlockModelSubmits` obtains the ordinary render
  buffer and, when `outlineColor != 0`, an `OutlineBufferSource` buffer for the
  same render type. Each baked quad is written to the ordinary buffer first and
  then to the outline buffer.
- `OutlineBufferSource.getBuffer` maps an affects-outline render type to its
  outline render type. `EntityOutlineGenerator` replaces vertex color with the
  selected outline color and ignores the source color, light, overlay, and
  normal attributes.
- `SnowGolemHeadLayer` and `MushroomCowMushroomLayer` use
  `submitOnlyOutline` only for the invisible-and-glowing branch. Their ordinary
  visible branch calls `submit`, so a nonzero outline color produces both
  surfaces.
- `IronGolemFlowerLayer`, `CarriedBlockLayer`, `BlockDecorationLayer`,
  `AbstractMinecartRenderer`, `TntMinecartRenderer`, and `TntRenderer` call the
  ordinary `submit` path. Minecart and TNT contents therefore do not become
  outline-only merely because the entity is invisible and glowing.

Authoritative local sources are
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/block/BlockModelRenderState.java`,
`net/minecraft/client/renderer/feature/BlockFeatureRenderer.java`,
`net/minecraft/client/renderer/OutlineBufferSource.java`,
`net/minecraft/client/renderer/Sheets.java`,
`net/minecraft/client/renderer/rendertype/RenderTypes.java`,
`net/minecraft/client/renderer/entity/layers/SnowGolemHeadLayer.java`,
`MushroomCowMushroomLayer.java`, `IronGolemFlowerLayer.java`,
`CarriedBlockLayer.java`, `BlockDecorationLayer.java`,
`net/minecraft/client/renderer/entity/AbstractMinecartRenderer.java`,
`TntMinecartRenderer.java`, and `TntRenderer.java`.

## Ownership and frozen interfaces

- `bbb-native::item_models::dropped` remains the semantic owner of attachment
  discovery, blocks-atlas quad resolution, and resident frame submission.
  `EntityBlockAttachment::outline_only` continues to distinguish only an
  actual Vanilla `submitOnlyOutline` caller from an ordinary `submit` caller.
- Each non-empty solid or translucent geometry class contributes exactly one
  static `ItemModelAssetRequest`. An ordinary attachment contributes its
  existing `BlockFeature` instance and, when `outline_color != 0`, a second
  instance for the same identity/atlas/geometry owner with
  `ItemModelDrawTarget::EntityOutline`, `ItemModelDrawPhase::Outline`, the
  unchanged transform, and normalized outline tint.
- The ordinary instance keeps its existing solid/translucent phase, white
  instance tint, light, and overlay. The outline copy carries the same light
  and overlay as inert metadata; the outline shader continues to use only the
  blocks-atlas alpha mask and opaque outline RGB.
- `bbb-renderer::item_models::residency` keeps phase and target outside
  `ItemModelAssetKey`. Its existing resident outline pipeline, shared entity
  outline target, content predicate, and post chain consume both
  `submitOnlyOutline` instances and ordinary outline-copy instances without a
  second geometry owner or a new render target.

## Bounds and fallback rules

- A zero outline color emits only the ordinary block-feature instance. Missing
  or empty model geometry drops the attachment exactly as before.
- An outline-only snow-golem or mooshroom attachment emits only outline
  instances and still drops a zero-color request. It never regains a main
  block-feature surface.
- Solid and translucent owners each receive an outline copy when present. The
  outline copy always uses the common no-cull, depth-writing, `LessEqual`
  blocks-atlas outline pipeline rather than inheriting the ordinary blend
  phase.
- Falling blocks and moving pistons retain their current zero outline color and
  moving-block paths. No entity outline copy is inferred for them.
- Existing resident owner/byte caps, per-frame instance bounds, atlas
  generation validation, target clear rules, and post-chain activation
  predicate remain authoritative.

## Non-goals

- This slice does not change attachment transforms or the existing visibility
  policy of iron-golem, enderman, or copper-golem layers.
- It does not add outline copies for held item models, item frames, block
  displays, special item renderers, textured entity models, block entities, or
  terrain/moving-block submissions.
- It does not redesign entity-outline shaders, Sobel/blur/composite passes,
  target sizing/depth, sampler/mipmap policy, or ordinary block-feature
  ordering.
- It does not close the generic fine-grained GPU render-state or remaining
  mob-specific armor umbrellas.

## Serial stages and verification

1. **Native dual submission.** Reuse each resident geometry owner for its
   ordinary main instance and conditional outline-copy instance, keep the
   hidden snow-golem/mooshroom branch outline-only, and restore ordinary
   minecart/TNT semantics. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native entity_block` and
   `scripts/cargo-dev.sh test -p bbb-renderer block_attachment`.
2. **Renderer consumption proof.** Lock owner reuse, phase/target filtering,
   shared-target activation, pass ordering, and deterministic offscreen
   consumption of an ordinary main-plus-outline pair. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-renderer entity_outline` and
   `scripts/cargo-dev.sh test -p bbb-renderer resident_item_model`.
3. **Closeout.** Re-run all focused families, remove the ledger item, archive
   this contract, append the milestone index, run the standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`98632f57bb4da13a1fdfbbfd23bf63d3f96fff3c`. The contract, native, and renderer
stages are the clean commits `4b8b89696`, `21e21afc4`, and `40360558e`. Any
focused or gate failure is fixed forward before another slice; history is not
rewritten.
