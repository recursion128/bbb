# Sign Text Rendering Parity Engineering Contract

Status: completed on 2026-07-29.

## Goal and completion

Close the remaining Vanilla 26.1 sign-face text rendering gaps as one coherent
renderer engineering slice:

- redraw glowing text through the eight neighbouring outline offsets when the
  black-text or camera-distance/scoping rule enables it;
- render underline and strikethrough effect bars with the font provider's white
  effect material;
- replace each non-space obfuscated glyph with a per-frame random glyph from
  the same advance bucket without changing layout; and
- draw primary glyphs and effect bars through the exact text polygon-offset
  depth state while keeping outline glyphs on normal text depth.

Completion requires deterministic layout/random/outline geometry tests, native
frame-context wiring tests, pipeline-state and pass-order tests, removal of the
active ledger item, archival of this contract, a concise milestone index entry,
and a passing full gate at the final head.

## Authoritative behavior

- `AbstractSignRenderer.extractRenderState` stores
  `isOutlineVisible(blockPos)`. That predicate is true when the local player is
  both first-person and scoping; otherwise the active camera entity must be
  strictly closer than `16^2` to the block centre.
- Only glowing text can draw an outline. Glowing black text always enables it;
  other glowing colors use the extracted visibility predicate. The outline
  color is `getDarkColor`: black glowing text uses `0xFFF0EBCC`, while other
  dyes scale each RGB channel by `0.4`.
- `Font.drawInBatch8xOutline` visits offsets in nested order
  `x=-1..1`, `y=-1..1`, excluding `(0,0)`. Each offset uses the glyph's
  one-pixel shadow offset, overrides every run color with the outline color,
  and renders through `DisplayMode.NORMAL`.
- The primary text keeps its base/per-run colors and uses
  `DisplayMode.POLYGON_OFFSET`. Vanilla's `TEXT_POLYGON_OFFSET` depth state is
  `LessEqual`, depth-write enabled, slope bias `-1`, constant bias `-10`.
- `Font.getGlyph` replaces each obfuscated non-space codepoint with
  `FontSet.getRandomGlyph(random, ceil(originalAdvance))`. Pen advance and
  effect bars still use the original glyph. Outline passes and the primary pass
  each perform their own lookup in submission order.
- For each styled codepoint, strikethrough covers
  `[effectX0, x+advance] x [y+3.5, y+4.5]`; underline covers the same horizontal
  range at `[y+8, y+9]`. `effectX0` is one pixel left of the first codepoint and
  otherwise equals `x`. Effects use depth `0.01`, the text color, the provider's
  white effect glyph, and the same polygon-offset display mode as primary text.
- Sign text still uses the standing/hanging renderer's face translation and
  font-pixel scale. That model transform is independent of, and must remain in
  addition to, the GPU polygon-offset depth state.

Authoritative local sources are
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/blockentity/AbstractSignRenderer.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/Font.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/font/FontSet.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/font/glyphs/BakedSheetGlyph.java`,
`/home/zgy/Work/mc-code/sources/26.1/com/mojang/blaze3d/font/GlyphInfo.java`,
and
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/RenderPipelines.java`.

## Ownership and frozen interfaces

- `bbb-native::sign_scene` continues to resolve filtered sign Components,
  standing/hanging transforms, base colors, and light. Stable block-entity
  extraction continues to cache `SignTextSurface` values by block-entity and
  filtering revision.
- `SignTextSurface` changes from a permanently baked mesh to an immutable,
  measured four-line presentation: original glyph, pen position, resolved
  style/color, line-first marker, transform, light, and outline metadata.
  Public count/empty introspection remains narrow; no protocol or world type
  enters the renderer.
- `bbb-renderer::item_models::sign_text` owns the per-frame geometry walk. It
  consumes the existing default-font `HudObfuscatedGlyphPool` and renderer frame
  index, preserving original advances while selecting equal-advance bitmaps.
  One deterministic random walk follows visible surface, line, outline-offset,
  then primary submission order.
- The collector returns three explicit geometry batches: normal-depth outline
  glyphs, polygon-offset primary glyphs, and polygon-offset effect bars.
  Primary/outline batches sample the uploaded default-font atlas. Effect bars
  sample a renderer-owned opaque white 1x1 material stored beside that atlas.
- A new bounded value-type `SignTextOutlineContext` carries only
  `camera_entity_position: Option<[f32; 3]>` and
  `first_person_scoping: bool`. Native derives the former from the interpolated
  undetached camera-entity pose (entity position, not detached camera eye) and
  the latter from camera type plus local scoping state, then applies it with one
  renderer setter each frame.
- The render pass draws outline geometry first with the existing normal,
  alpha-blended, no-cull text depth pipeline. It then draws primary glyphs and
  effect bars with the existing text polygon-offset pipeline, switching the
  atlas binding only for the effect batch. Sign geometry remains in the
  order-zero entity feature location.

## Bounds and fallback rules

- The existing stable sign/block-entity and visible-position bounds remain the
  only collection bounds; this slice adds no persistent queue or canonical
  state.
- Missing font atlas still drops sign text. A missing same-advance obfuscation
  bucket falls back to the original glyph. Spaces are never substituted.
- Missing/non-finite camera-entity position disables the distance branch.
  `first_person_scoping` still enables outlines, matching Vanilla's separate
  local-player fast path. Non-glowing faces never outline.
- The deterministic frame seed replaces Vanilla's process-random `Font.random`
  seed while preserving per-frame cycling, advance buckets, lookup count, and
  submission ordering. This is the repo's established reproducible visual
  random policy.
- Empty glyphs retain their advance and may still emit styled effect bars.
  Empty faces remain absent.

## Non-goals

- This slice does not add custom Component-font identity to sign protocol
  projection, unihex/vector glyph providers, text shadows, sign editing UI, or
  resource-pack atlas redesign.
- It does not change sign wrapping, bidi completion, positioning, dye
  resolution, filtering, light selection, block-entity visibility, or body
  models except where a regression test must preserve them.
- It does not generalize polygon offset, white-effect materials, or dynamic
  obfuscation to unrelated item labels or entity text.
- It does not add a live playable fixture; deterministic semantic, pipeline,
  and geometry evidence is the acceptance surface.

## Serial stages and verification

1. **Prepared text and dynamic primary geometry.** Replace stable baked meshes
   with measured glyph slots, add per-frame equal-advance substitution, emit
   exact underline/strikethrough rectangles, add the white effect material, and
   route primary/effect draws through the polygon-offset pipeline. Focused test:
   `scripts/cargo-dev.sh test -p bbb-renderer sign_text`.
2. **Outline frame context and geometry.** Add the native-derived outline
   context, strict block-centre distance/scoping rule, black-glowing override,
   eight ordered normal-depth redraws, and render ordering. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native sign` and
   `scripts/cargo-dev.sh test -p bbb-renderer sign_text`.
3. **Closeout.** Re-run both focused families, update the active ledger and
   archive/index documentation, perform standard per-commit checks, review the
   integrated diff, and run the final full gate.

The fully gated `3e0737026` is the pre-start high-risk checkpoint. Each stage is
buildable and independently reviewable. Failures localize to prepared layout
and deterministic substitution, effect material/depth routing, native outline
context, or outline geometry/pass order, and are fixed forward before the next
stage.
