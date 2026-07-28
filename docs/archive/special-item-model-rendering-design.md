# Special Item Model Rendering Closure

This document freezes the engineering contract for executing Vanilla 26.1
`minecraft:special` item models. Work starts from full-gated commit
`c815956a2`.

## Goal and Completion Standard

The completed path is:

```text
selected client-item definition + stack/context properties
  -> ordered ordinary/special leaves with composed model transformations
  -> stack-component argument extraction
  -> renderer-owned special geometry and entity-texture residency
  -> GUI, first-person, dropped-item, and item-frame submissions
```

All 16 entries registered by `SpecialModelRenderers.bootstrap` must parse into
typed values and execute through their matching geometry owner. The base baked
model supplies `ModelRenderProperties` only; it must never replace the special
geometry. Conditions, selects, ranges, and composites preserve the selected
leaf order and the parent-to-child transformation composition.

Completion requires focused resource-pack, item-runtime, native, and renderer
tests for every registered type; explicit GUI, first-person, dropped-item, and
item-frame path tests; component-backed banner, shield, player-head, and
decorated-pot tests; reverse checks against the local Vanilla bootstrap; removal
of the matching unsupported-feature todo; a full gate; and the applicable
playable smoke at current HEAD.

## Non-Goals

- Dynamic item tint parity and equipment asset layers remain separate resource
  coverage slices.
- Special models do not add block-entity text, container UI, animation, or
  server authority. Item signs render their model only; item banners do not
  simulate world wind; fixed book/chest/shulker openness comes from the model
  JSON, not a new tick owner.
- `base` model particle materials remain available to particle/material queries,
  but are not visible item geometry.
- No in-game resource-pack or rendering configuration UI is added.

## Owners and Frozen Boundaries

- `bbb-pack` owns typed, validated unbaked special-model definitions and direct
  texture references. Unknown renderer ids and invalid enum/non-finite values
  fail at load time; field spelling and defaults follow the Vanilla codec.
- `bbb-item-model` owns client-item tree baking and value-aware leaf selection.
  Its resolved output is an ordered list of ordinary or special leaves carrying
  one composed finite transformation. Existing icon/material projections remain
  compatibility views over that one result, not an independent selector.
- `bbb-protocol` already owns the stack component summaries. No wire change is
  planned. Banner patterns, base colour, profile, and pot decorations are read
  only from the effective stack component patch.
- `bbb-native` owns display-context/owner inputs and the final world, hand,
  item-frame, and GUI placement transforms. It must ask the item runtime before
  taking the same-id block-item shortcut, because a selected special leaf
  supersedes ordinary block geometry.
- `bbb-renderer` owns special geometry, entity texture atlas resolution, GPU
  residency, render phases, extents-based GUI fitting, and foil submission. One
  special asset source joins the existing item-model residency keys; no second
  item renderer or native-owned entity-texture atlas is introduced.
- `bbb-world` remains canonical stack and registry state. It stores no model
  geometry, renderer handle, or resource path.

The renderer-facing asset identity includes resource generation, special type,
all static JSON parameters, stack-derived geometry arguments, and geometry
part. Frame-varying placement, light, overlay, foil, and target remain instance
data. Reload replaces the entity atlas and special resident owners within the
existing renderer transaction.

## Vanilla 26.1 Evidence

Authoritative local sources are under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer`.

- `item/SpecialModelWrapper.java` composes the inherited and local
  transformations, reads render properties from `base`, creates one special
  layer, extracts the stack argument, and applies standard foil.
- `special/SpecialModelRenderers.java` registers exactly bed, bell, banner,
  book, conduit, chest, copper-golem statue, head, player head, shulker box,
  shield, trident, decorated pot, standing sign, hanging sign, and end cube.
- Each `Unbaked.bake` binds its model layer and texture. No-data renderers
  extract `null`; banner reads `BANNER_PATTERNS`, player head reads `PROFILE`,
  shield reads the immutable component map, and decorated pot reads
  `POT_DECORATIONS`.
- `DecoratedPotSpecialRenderer` passes missing decorations as
  `PotDecorations.EMPTY`. `DecoratedPotRenderer` consumes the four faces in
  back, left, right, front order.
- `StandingSignSpecialRenderer`'s Vanilla codec field is spelled
  `attachement`; resource-pack compatibility must preserve that spelling.
- `SpecialModelRenderer.getExtents` supplies GUI bounds. Special item GUI
  lighting uses the entity-in-UI lane; ordinary world/hand light and overlay
  remain caller supplied.

## Ordered Stages, Tests, and Checkpoints

### A. Typed pack and resolved-leaf contract

Replace the raw special payload with exhaustive typed variants, retain the raw
JSON only where diagnostics need it, reject ids outside the Vanilla bootstrap,
and expose direct texture references. Add the one ordered resolved-leaf tree
that carries composed model transformations without changing downstream
rendering yet.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-pack item_model
scripts/cargo-dev.sh test -p bbb-item-model item_runtime
```

This is the first safe stop. The compatibility icon projection may still use
the base material, but production leaf selection and transformation evidence
must be complete and tested. Commit and run the full gate before changing GPU
residency.

### B. Renderer special asset residency

Add the entity-atlas special asset source, static geometry builders for all 16
registered types, GUI/world camera bind-group selection, extents fitting, and
standard foil. Reuse existing block-entity/entity model layers where their
geometry is identical; add narrow shield, statue, or special-only geometry
owners where it is not.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-renderer special_item
scripts/cargo-dev.sh test -p bbb-renderer item_model_residency
```

The renderer API and all special builders land together. This is the second
safe stop and a full-gate checkpoint; native does not publish special requests
before the renderer can consume every registered type.

### C. Native path and component integration

Resolve special leaves before the block-item shortcut, extract stack arguments,
and submit the same asset contract from GUI, first person, dropped items, and
item frames. Preserve composite ordering, item clusters, handed display
transforms, light/overlay, map precedence, and existing flat/block fallbacks.
Load pack-defined direct special textures into the accepted entity atlas.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-native item_models
scripts/cargo-dev.sh test -p bbb-native hud
scripts/cargo-dev.sh test -p bbb-item-model special_item
scripts/cargo-dev.sh test -p bbb-renderer special_item
```

This stage is not safe to split between producers and consumers. Commit only
after all four targets and component-backed variants pass.

### D. Reverse closure and final gate

Reverse-check the 16 bootstrap registrations and all local official special
assets, remove the unsupported-feature todo, append the engineering closure to
the milestone index, and run:

```text
scripts/cargo-dev.sh gate
scripts/playable-gate.sh auto
scripts/audit-p0.sh
```

The final HEAD must be clean and fully gated. Failures localize first to pack
typing/selection, then renderer residency, then the four native producers.
Fixes move forward; a failed stage never publishes a half-wired boundary.
