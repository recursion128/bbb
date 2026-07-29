# Block-feature ordered submission closure

This document freezes the engineering contract for closing the remaining
Vanilla 26.1 block-feature submission and ordered world-text gap. Work starts
from full-gated commit `db2c6224a`.

## Goal and completion standard

The completed translucent feature sequence is:

```text
order 0 model/name/text
  -> order 0 item features
  -> order 0 moving/block/breaking block features
  -> order 0 custom geometry
  -> order 1 text
  -> translucent terrain and particles
```

Solid block-model and moving-block submissions likewise draw after solid item
features. Ordinary block displays, item-frame borders, entity-attached block
models, falling blocks, and moving pistons must publish through an explicit
block-feature lane rather than depending on append order inside the world item
lane. Existing block-destruction overlays remain the breaking-block lane.

Completion requires source-backed coverage of all three
`BlockFeatureRenderer` node families, exact order metadata on map/display text
custom geometry, renderer source assertions for both solid and translucent
ordering, focused native/renderer tests, removal of the matching unsupported
feature, a full gate, and the applicable playable smoke at the final head.

## Non-goals

- This slice does not add a new block-state/model loader, custom registry
  mapping, block-entity renderer, item special-model type, shader, atlas, or
  direct textured-mesh path.
- It does not change world block/entity state, model transforms, tint, light,
  overlay, outline eligibility, crumbling geometry, or target-combine math.
- It does not generalize every non-zero ordered entity-model layer. The
  existing model-feature buckets remain authoritative; this slice closes the
  concrete block/custom/text dependency named by the ledger.
- First-person maps and GUI item rendering keep their private passes and are
  not world feature submissions.

## Owners and frozen boundaries

- `bbb-world` remains the canonical owner of falling-block, moving-piston,
  block-display, item-frame, and attachment state. No world API change is
  planned.
- `bbb-native` assigns the semantic renderer lane while producing the existing
  immutable `RendererFrame`. The block display, item-frame border, and
  `entity_block_models` producers are the only lane changes.
- `bbb-renderer` owns the lane value, resident filtering, frame-pass order, and
  order-zero/order-one display-text partition. It keeps map/display texture,
  render type, tint, transform, light, overlay, outline, `order`, and
  `submit_sequence` metadata before atlas lookup.
- Existing resident assets remain the single geometry/residency owner.
  `BlockFeature` is a submission lane, not a second cache or atlas.

## Vanilla 26.1 evidence

Authoritative sources are under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer`.

- `FeatureRenderDispatcher.renderSolidFeatures` visits item features before
  block features for every ordered collection.
- `renderTranslucentFeatures` visits name tags, text, items, block features,
  then custom geometry for each ordered collection, and processes collections
  in ascending order.
- `BlockFeatureRenderer` has exactly `MovingBlockSubmit`,
  `BlockModelSubmit`, and `BreakingBlockModelSubmit`. Falling blocks and piston
  heads submit moving blocks; `BlockModelRenderState` supplies the ordinary
  block-model users; `LevelRenderer.submitBlockDestroyAnimation` supplies the
  breaking family.
- `MapRenderer` submits the textured map and decoration geometry at order zero
  and decoration labels through `order(1).submitText`.
- `DisplayRenderer.TextDisplayRenderer` submits an optional background as
  order-zero custom geometry, then uses order one for text only when that
  background exists. Text without a background remains order zero.
- `AbstractSignRenderer` uses the default order-zero `submitText`.

## Serial stages and verification

### A. Explicit block-feature lane

Add one resident `BlockFeature` lane, route block displays, item-frame borders,
entity block attachments, falling blocks, and moving pistons into it, and draw
it after the item lane in solid and translucent feature paths. Preserve the
existing breaking overlay pass.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-native block_feature
scripts/cargo-dev.sh test -p bbb-native item_frame
scripts/cargo-dev.sh test -p bbb-renderer item_model_residency
scripts/cargo-dev.sh test -p bbb-renderer block_feature
```

This is a buildable safe stop. A failure localizes to producer classification
or resident target filtering; text/custom ordering has not moved yet.

### B. Ordered custom geometry and text

Partition display glyph geometry by its recorded order. Keep sign and
background-free display text in order zero; move world map base/decorations
and display backgrounds after all block features; draw map labels and
background-backed display text in order one. Update `FRAME_STEPS` and semantic
source assertions.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-renderer display_text
scripts/cargo-dev.sh test -p bbb-renderer item_frame_map
scripts/cargo-dev.sh test -p bbb-renderer block_feature
```

This is the engineering checkpoint because it crosses the render-graph order
boundary. Run the full gate before ledger closeout if focused behavior is
abnormal or the stage reaches four commits.

### C. Reverse closure

Add a Vanilla-source guard that pins the three block node families and the
MapRenderer/TextDisplay/Sign order choices, remove the ledger item, archive
this contract, append the milestone index, and run:

```text
scripts/vanilla-source-tools.sh python -m unittest \
  scripts.tests.test_block_feature_order_partition
scripts/cargo-dev.sh gate
scripts/playable-gate.sh auto
scripts/audit-p0.sh
```

The final head must be clean and fully gated. Failures are fixed forward
without rewriting history.
