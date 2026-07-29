# Rich Component Presentation Design

Engineering contract for closing the remaining trusted-Component presentation
gap in chat, book hover effects, and item tooltips without introducing a second
Component or item-stack schema.

## Goal and completion standard

The native client must consume the exact font and hover payloads already retained
by `bbb-protocol` so that:

- chat and read-only book text select each resolved Component run's resource-pack
  font for measurement, wrapping, hit geometry, obfuscation, and draw geometry;
- unavailable font ids fall back to the active default font, as the existing
  `HudFontGlyphBook` contract requires;
- `show_item` creates the retained `ItemStackTemplate` against the current frozen
  registries and feeds that `ItemStackSummary` through the one native item
  tooltip builder;
- `show_entity` shows no tooltip unless advanced item tooltips are enabled, then
  shows optional name, localized `Type: <entity description>`, and canonical UUID
  lines in Vanilla order; and
- `show_text` keeps its resolved styles and font ids while wrapping to Vanilla's
  `max(guiWidth / 2, 200)` bound.

Completion requires bounded sanitization, font-aware chat/book wrap and hit
tests, reusable multi-line tooltip values, focused world/native/renderer and
offscreen proof, deletion of the two now-complete rich Component/font TODOs,
an archived contract and milestone index entry, a clean reviewed diff, and a
fully gated final HEAD.

## Authoritative 26.1 references

Minecraft sources under `~/Work/mc-code/sources/26.1/`:

- `net.minecraft.network.chat.HoverEvent`
- `net.minecraft.network.chat.Style`
- `net.minecraft.client.gui.GuiGraphicsExtractor`
- `net.minecraft.client.gui.Font`
- `net.minecraft.client.gui.font.FontSet`
- `net.minecraft.client.gui.font.glyphs.EmptyGlyph`
- `net.minecraft.client.gui.screens.ChatScreen`
- `net.minecraft.client.gui.screens.inventory.BookViewScreen`
- `net.minecraft.world.item.ItemStackTemplate`
- `net.minecraft.world.item.ItemStack`

## Frozen behavior and ownership

- `HoverEvent.ShowItem` carries an `ItemStackTemplate`: resource id, count in
  `1..=99`, and a persistent data-component patch. `GuiGraphicsExtractor`
  creates the stack at hover time and sends it through the ordinary item
  tooltip path.
- `HoverEvent.EntityTooltipInfo.getTooltipLines` emits optional name first,
  translated type second, and UUID third. `GuiGraphicsExtractor` exposes these
  lines only while advanced item tooltips are enabled.
- `show_text` uses `Font.split(text, max(guiWidth / 2, 200))`.
- `Font` resolves every codepoint and its advance through
  `style.getFont()`. Obfuscation chooses an equal-advance glyph from the same
  `FontSet`. A missing resource font falls back through the client font owner;
  bbb's existing `HudFontGlyphBook` provides that fallback.
- A `space` provider produces an `EmptyGlyph`: it contributes advance and
  optional underline/strikethrough effects but no textured glyph quad. The
  existing item-model font baker and renderer geometry are the only owners of
  this behavior.
- `bbb-protocol` remains the sole canonical Component schema. Its existing
  resolved hover-event projection gains the exact nested `show_text` value and
  `show_entity` name NBT already present in the owning Component, so native does
  not need to rediscover a nested payload by run index.
  `bbb-world::client::component_hover` owns registry-aware projection of a
  retained `show_item` payload into `ItemStackSummary`.
  `bbb-native::runtime::component_hover` owns localization and conversion to
  reusable renderer tooltip values.
  `bbb-renderer` owns font-aware wrapping, hit geometry, tooltip placement, and
  draw ordering.

## Explicit non-goals

- Server-supplied `open_file` remains deliberately inert.
- No in-game configuration UI is added; the existing startup/debug advanced
  tooltip option remains the authority.
- Unihex/unifont, bidi shaping, glyph kerning, and non-bitmap font providers
  remain governed by their existing deferred ledger entries. This slice uses
  the selected bitmap/space font book and does not claim those providers.
- This slice does not add new data-component decoders. Unsupported persistent
  item components keep the existing bounded `show_item` fallback instead of
  being guessed.
- No renderer or world state retains GPU handles, filesystem paths, or a second
  mutable item-tooltip cache.

## Serial stages, tests, and checkpoints

Each stage is a buildable safe stop. The baseline `4dd9934e9` is fully gated.
This is a cross-crate renderer-value and hit-geometry engineering slice; run a
full gate after stages 2 and 4, at four accumulated commits, and immediately if
focused behavior becomes abnormal.

1. **Exact hover projection and reusable tooltip values.** Retain the exact
   nested Component NBT beside the existing resolved `show_text`/`show_entity`
   summary runs, add canonical UUID formatting, the narrow world `show_item`
   projection, renderer multi-line tooltip vocabulary, and a native
   Component-hover presenter that delegates item lines to `hud_item_tooltip`.
   Preserve a bounded id/count fallback when the item template cannot be
   projected.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-protocol component_hover`,
   `scripts/cargo-dev.sh test -p bbb-world component_hover` and
   `scripts/cargo-dev.sh test -p bbb-native component_hover`.
2. **Font-aware chat pipeline.** Carry exact font ids through chat source runs,
   cached wrapped chars/lines, interaction ranges, rendering, tooltip wrapping,
   and resource-reload rebuilds. Gate entity tooltip visibility from the
   per-frame advanced-tooltip option.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-renderer hud_chat` and
   `scripts/cargo-dev.sh test -p bbb-native chat_component`.
3. **Read-only book parity and shared tooltip rendering.** Move read-only book
   Component runs and interactions through the same font-aware wrap/hit and
   multi-line tooltip path. Editable/signing plain inputs remain default-font
   text.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-renderer hud_book` and
   `scripts/cargo-dev.sh test -p bbb-native book_screen`.
4. **Evidence and closeout.** Add deterministic custom-font, missing-font,
   space-advance, style-effect, item/entity/show-text, advanced-gate, reload,
   sanitization, and offscreen pixel tests. Re-audit the native inventory
   tooltip TODO against the already shared font baker/style geometry and add
   only any missing evidence needed for deletion. Close the ledger items,
   archive this contract, update the milestone index, run P0/coverage audits,
   review, commit, and gate.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-item-model font`,
   `scripts/cargo-dev.sh test -p bbb-renderer rich_component`,
   `scripts/cargo-dev.sh test -p bbb-renderer offscreen_chat`, and
   `scripts/cargo-dev.sh test -p bbb-native component_hover`.

Failures are fixed forward within the current stage. If a retained persistent
component cannot be projected by the existing registry codec, the safe stop is
the documented id/count fallback plus a focused test; this slice does not widen
the data-component codec.
