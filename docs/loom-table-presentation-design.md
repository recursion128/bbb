# Loom Table Presentation Engineering Contract

Status: active engineering contract.

## Goal and completion

Close the active `LoomMenu` presentation gap with the exact observable Vanilla
26.1 screen behavior:

- draw the ordered selectable-pattern thumbnails in the visible four-by-four
  grid, preserving selection, hover, scrolling, and synchronized registry
  order;
- localize the hovered pattern name from its current-session registry
  presentation and the selected dye color;
- render the result banner's base color and complete ordered pattern stack in
  the result rectangle;
- suppress pattern selection at Vanilla's six-pattern limit and render the
  max-pattern error overlay; and
- preserve Vanilla's extraction strata: base and empty-slot sprites, scroller,
  result preview or error, then each button sprite immediately followed by its
  thumbnail.

Completion requires semantic tests for empty, selectable, selected, hovered,
scrolled, max-pattern, missing-resource, static-pattern, and custom dynamic
pattern states; renderer command-order tests; deterministic offscreen pixel
readback for static and dynamic pattern thumbnails plus the result banner;
removal of the active ledger item; archival of this contract; a concise
milestone index entry; and a passing full gate at the final head.

## Authoritative behavior

- `LoomScreen.containerChanged` reads the result stack's `BANNER_PATTERNS`,
  marks the input as full when its effective pattern count is at least six, and
  shows the grid only when banner and dye inputs are nonempty, the input is not
  full, and the menu exposes selectable patterns.
- `LoomScreen.extractBackground` draws the 176x166 background, empty banner,
  dye, and pattern slots, the scroller at `(119,13)` with
  `floor(41 * scrollOffs)`, then either the result preview or the error sprite
  at `(138,52)` with size 26x26.
- The pattern grid starts at `(60,13)`, has four columns and four rows of
  14x14 buttons, and advances in synchronized selectable-holder order. The
  selected button sprite wins over hover. A nonselected hovered button uses
  the highlighted sprite, pointer cursor, and the localized
  `<pattern translation key>.<dye serialized name>` tooltip.
- Each thumbnail is extracted immediately after its button: translate by
  `(4,2)`, fill a 5x10 gray banner base, then sample the pattern sprite's
  64x64 atlas region with horizontal range `0..21` and vertical range `1..41`.
- `GuiBannerResultRenderer` draws a 20x40 PIP at `(141,8)`, uses
  `Lighting.Entry.ITEMS_FLAT`, full-bright light, scale 16, and
  `translate(0,0.25,0)`. It renders only the `BannerFlagModel`: the base flag,
  a base-color pattern pass, then at most sixteen ordered pattern passes. It
  does not render the world banner pole or frame.

Local sources under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/inventory/LoomScreen.java`,
`net/minecraft/client/gui/render/pip/GuiBannerResultRenderer.java`,
`net/minecraft/client/gui/render/GuiGraphicsExtractor.java`, and
`net/minecraft/client/renderer/blockentity/BannerRenderer.java` are
authoritative.

## Ownership and frozen interfaces

- `bbb-world` remains the canonical owner of the open container, effective
  item stacks, `BANNER_PATTERNS`, synchronized selectable-pattern holder order,
  holder presentation, and item profiles. It gains no screen cursor or GPU
  state.
- `bbb-native` remains the sole world-to-render projection owner. Its Loom
  projection derives `display_patterns`, `has_max_patterns`, visible holder
  range, selected/hovered state, localized tooltip, thumbnail descriptors, and
  optional result-banner preview from canonical state plus screen-local scroll
  and pointer state.
- `bbb-native::banner_scene` owns one shared current-session pattern texture
  resolver. World banners, Loom thumbnails, and the Loom result preview must
  consume that resolver rather than introduce another asset-id-to-texture fact
  source. A synchronized holder resolves only through its current-session
  presentation; bootstrap ordinal fallback must not override synchronized
  registry identity.
- `bbb-renderer::HudInventoryScreen` gains a bounded vector of typed banner
  thumbnail descriptors. Each descriptor carries the existing
  `BannerPatternTexture`, exact screen-relative rectangle/UV crop, and the
  background-layer insertion point that follows its button.
- The entity renderer exposes HUD bind groups for its existing static model
  texture atlas and bounded dynamic entity texture atlas. HUD pattern commands
  select one of those resident atlases and reuse its UV allocation; they do not
  decode, upload, or retain duplicate images.
- `EntityModelKind::Banner` gains an explicit flag-only form and a narrow
  constructor for GUI result banners. World banner constructors retain their
  pole/frame pass; the GUI form emits only the flag and ordered pattern passes.
- The existing isolated entity-preview PIP target renders the result at
  `(141,8,20,40)` with scale 16, translation `(0,0.25,0)`,
  `ItemsFlat`, full-bright light, and an insertion point after the scroller and
  before pattern buttons. Static and dynamic pattern layers continue through
  the existing banner render submissions and texture residency owners.

## Fallback rules

1. A missing item runtime derives banner and dye identities only from
   resource-derived world item profiles and effective components; it never
   embeds protocol ids.
2. Missing synchronized holder presentation omits that thumbnail image and
   tooltip identity while retaining its ordered button. It does not reuse a
   bootstrap holder at the same ordinal.
3. A builtin pattern missing from the static atlas, or a custom pattern whose
   bounded dynamic texture is pending or failed, retains the gray thumbnail
   base and records the existing submission-first diagnostic path.
4. Missing result banner identity or components omit the PIP without
   fabricating a color or pattern stack. The static screen and slots remain
   visible.
5. An input at or above six effective patterns always suppresses the grid and
   result preview and shows the max-pattern error, independent of stale local
   selection or scroll state.

## Non-goals

- This slice does not change packet decoding, selectable-pattern
  synchronization, menu-button commands, scroll/click routing, result-stack
  prediction, or server authority.
- It does not change world banner animation, pole/frame geometry, block-entity
  state, or the separate banner depth-write parity item.
- It does not generalize every GUI PIP producer, add a standalone texture
  cache, or refactor unrelated entity render-state buckets.
- It does not close Beacon, Lectern, Smithing, or other container-menu gaps and
  does not add in-game configuration UI.

## Serial stages and verification

1. **Pure Loom projection and shared resource contract.** Add the shared
   current-session pattern resolver, exact visibility/max/hover/selection/
   tooltip projection, typed thumbnail and result descriptors, and focused
   native tests. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native loom`,
   `scripts/cargo-dev.sh test -p bbb-world loom`, and
   `scripts/cargo-dev.sh test -p bbb-renderer banner`.
2. **Resident atlas HUD draw and result PIP.** Add shared HUD bindings for the
   existing static/dynamic entity atlases, interleaved thumbnail commands,
   flag-only result rendering, command-order tests, and deterministic static
   plus dynamic offscreen readback. Remove the ledger item, archive this
   contract, and append the concise milestone index. Run the same focused tests
   plus the exact Loom offscreen tests, formatting, docs lint, diff-check, and
   full diff review, then commit clean and run the full gate.

The final full gate is the engineering checkpoint. A playable gate is required
only if input or container command routing changes; this presentation-only
slice otherwise uses structural interaction tests and deterministic GPU
readback as its acceptance surface.

## Failure localization and safe stops

Projection failures compare effective stacks, max-pattern state, synchronized
holder order, visible range, localization identity, and insertion indices
before renderer code. Thumbnail failures compare resolved
`BannerPatternTexture`, static/dynamic atlas residency, UV crop, then command
order and pixels. Result failures compare flag-only model selection, base
color, ordered layers, PIP transform/light, then offscreen pixels.

The clean contract commit and the buildable stage-1 commit are safe stops. Any
gate failure is fixed forward before another slice; history is not rewritten.
