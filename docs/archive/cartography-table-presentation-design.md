# Cartography Table Presentation Engineering Contract

Status: completed and archived on 2026-07-29.

## Goal and completion

Close the active `CartographyTableMenu` presentation gap with the exact Vanilla
26.1 result-preview behavior:

- retain the existing map, scaled, duplicated, locked, and error frame sprites;
- draw the current 128x128 dynamic map pixels inside every applicable result
  frame at Vanilla's exact origin and scale;
- draw the complete ordered map-decoration sprite and optional name layers over
  those pixels;
- preserve the duplicated-map `nextStratum` order between the two complete map
  previews; and
- derive a deterministic preview/error fallback when the native item runtime or
  canonical map state is temporarily unavailable.

Completion requires semantic projection tests for all four preview modes,
invalid inputs, missing item-runtime/map-state fallbacks, ordered decorations,
and names; renderer metadata plus deterministic offscreen pixel tests for map
pixels, decoration layering, duplicated strata, and locked overlay ordering;
removal of the active ledger item; and the required full gate at the milestone
head.

## Authoritative behavior

- `CartographyTableScreen.extractBackground` identifies the additional item as
  map, paper, or glass pane, reads the input `MAP_ID`, and marks paper invalid
  for a locked or scale-four map and glass pane invalid for a locked map.
- `CartographyTableScreen.extractResultingMap` fixes the result-frame and map
  transforms: scaled `(85,31,0.226)`, duplicated back
  `(86,16,0.34)` then front `(70,32,0.34)` after `nextStratum`, and ordinary or
  locked `(71,17,0.45)`, all relative to the 176x166 screen origin.
- `CartographyTableScreen.extractMap` submits pixels and decorations only when
  both the map id and saved map data exist. Missing data therefore retains the
  frame but emits no dynamic map surface.
- `MapRenderer.extractRenderState` and `MapRenderer.render` define the shared
  dynamic `minecraft:map/<id>` pixels, ordered decorations, optional decoration
  names, and per-decoration submit order. This slice reuses that behavior and
  the existing bbb map texture/decoration catalogs rather than implementing a
  second map renderer.
- `CartographyTableMenu.setupResultSlot` supplies the authoritative result
  fallback: scale and lock results carry `MAP_POST_PROCESSING`, duplication has
  count two, while an invalid paper/glass transform has an empty result.

Local sources under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/inventory/CartographyTableScreen.java`,
`net/minecraft/client/gui/MapRenderer.java`, and
`net/minecraft/world/inventory/CartographyTableMenu.java` are authoritative.

## Ownership and frozen interfaces

- `bbb-world::ItemProfiles` owns resource-derived protocol identities needed by
  inventory behavior. It gains an exact cartography additional-item profile
  for paper, empty map, and glass pane, populated atomically by native resource
  reload. This is the fallback authority when `NativeItemRuntime` is absent;
  production code must not add literal protocol ids.
- `bbb-native` remains the sole world-to-render projection owner. A
  `CartographyPreviewMode` pure projection selects frame layers, dynamic preview
  rectangles, invalid state, and fallback source from the open container,
  item-profile identities, result stack, and optional map state.
- `bbb-renderer::HudInventoryScreen` gains a bounded vector of typed map-preview
  entries. Each entry carries the map id, screen-relative origin and scale, and
  the background-layer insertion point needed to preserve Vanilla strata.
  Canonical map colors and decorations do not become renderer-owned state.
- `runtime/render_extract.rs` requests each visible cartography map through the
  existing `MapTextureExtractionCache` and projects decorations through the
  existing generated decoration catalog. The request joins the same per-frame
  map texture vectors used by first-person and item-frame maps; no parallel
  cache or upload path is allowed.
- The renderer extends its existing resident map texture, decoration, and font
  owners with a GUI target. HUD collection interleaves typed map draws with the
  declared inventory background insertion points. Map pixels, decorations, and
  names must use the same resident atlas identities and submission ordering as
  other map consumers.
- Missing dynamic texture residency records the existing submission-first
  fallback and leaves the static result frame visible. Unknown decoration types
  remain ignored by the generated catalog, matching the existing map paths.

## Fallback rules

The primary path uses exact resource-derived item identities plus canonical map
state. If an input is temporarily unavailable:

1. the result stack's `MAP_POST_PROCESSING` or count-two shape determines a
   valid scale, lock, or duplication mode;
2. a paper/glass transform is invalid only when canonical map state establishes
   Vanilla's locked or maximum-scale constraint; an empty result alone never
   turns missing map data into an invented error;
3. otherwise the screen keeps the ordinary map frame without inventing an
   error; and
4. absent canonical map state never fabricates pixels or decorations.

These rules keep the presentation deterministic and avoid guessing registry
ids or map content.

## Non-goals

- This slice does not change map packet decoding, canonical color/decorations
  storage, first-person maps, item-frame maps, or map texture cache limits.
- It does not add local cartography result-stack mutation or consume inputs;
  the server remains authoritative for container contents.
- It does not generalize unrelated standalone samplers, mip generation, or GPU
  render-state buckets.
- It does not add in-game configuration UI or close other container-menu gaps.

## Serial stages and verification

1. **Pure state and projection.** Add exact resource-derived additional-item
   identities, pure fallback selection, typed HUD map-preview descriptors, and
   exact mode/rect/stratum tests. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-world cartography`,
   `scripts/cargo-dev.sh test -p bbb-native cartography`, and
   `scripts/cargo-dev.sh test -p bbb-renderer cartography`.
   Commit the buildable stage and run a full gate because it changes the
   world/native/renderer integration surface.
2. **Resident GUI rendering and closeout.** Route visible previews through the
   existing map extraction cache, add the resident GUI target and ordered HUD
   draw commands for pixels/decorations/names, and add deterministic offscreen
   readback. Remove the active ledger item, archive this contract, and append a
   concise milestone index entry. Run the same focused tests, documentation and
   formatting checks, commit clean, then run the full gate and applicable
   playable gate.

Each stage also runs `cargo fmt --all --check`, `git diff --check`, full diff
review, and `scripts/docs-lint.sh` because the contract/ledger changes.

## Failure localization and safe stops

Projection failures compare mode, invalid state, rectangles, and insertion
points before inspecting renderer code. Texture failures compare map-cache
requests and resident map ids before bind groups or pixels. Decoration failures
compare generated type lookup and ordered descriptors before GPU geometry.
Offscreen failures compare map pixels, then decorations/names, then foreground
overlay order.

The clean full-gated stage-1 commit is the safe stop. Any gate failure is fixed
forward before stage 2; history is not rewritten.
