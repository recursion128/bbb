# Lectern Screen Presentation Engineering Contract

Status: completed on 2026-07-29.

## Goal and completion

Close the active `LecternMenu` presentation gap with the exact observable
Vanilla 26.1 book-view behavior:

- select written and writable raw or filtered pages from the client text
  filtering option;
- preserve rich Component styles, custom fonts, hover tooltips, and allowed
  click events;
- wrap and hit-test through the resident font book at Vanilla's 114-pixel
  width, nine-pixel line spacing, and fourteen visible-line limit;
- render only the currently visible normal or highlighted page controls and
  the correct one- or two-button Lectern footer; and
- route page, take-book, close, `change_page`, and `run_command` actions through
  the server-authoritative Lectern container without locally changing its page
  or book slot.

Completion requires semantic world tests for exact written Component NBT,
writable literal pages, filtering, empty access, clamped server page, and
revision changes; native projection and command-order tests for rich
style/font/hover/click, filtering, stale targets, page keys, page controls,
footer controls, and run-command close ordering; renderer layout/visibility/
hover tests plus deterministic offscreen pixels; removal of the active ledger
item and obsolete parallel inventory projection; archival of this contract; a
concise milestone index entry; and a passing full gate at the final head.

## Authoritative behavior

- `LecternScreen.bookChanged` calls `BookViewScreen.BookAccess.fromItem`, or
  uses `EMPTY_ACCESS` when slot zero has no supported book component.
  `BookAccess.fromItem` chooses written Components before writable literal
  strings and selects filtered alternatives only when global text filtering is
  enabled.
- `BookViewScreen.visitText` merges the page with black, no-shadow style, calls
  `Font.split` at width 114, and draws at `(left+36, top+30)` with line height
  nine and at most `128/9 == 14` lines. The right-aligned page indicator ends
  at `(left+148, top+16)` and uses at least one as the displayed page count.
- The 192x192 book background is top-centered at y=2. Back and forward
  controls are 23x13 at relative `(43,157)` and `(116,157)`, use highlighted
  sprites on hover, and are visible only when the synchronized current page
  has a predecessor or successor. Lectern page turns do not play the standalone
  book turn sound.
- PageUp and PageDown press those same page controls. Lectern sends container
  buttons 1 and 2; a `change_page` click sends `100 + (page - 1)` when different
  from the synchronized page. Neither path mutates the local menu first.
- Players with `mayBuild` receive 98x20 Done and Take Book buttons at relative
  x=-4 and x=98, y=194; other players receive the 200x20 Done button.
  Take Book sends button 3. Done and normal close send ContainerClose.
- Left-click rich text lookup uses the same wrapped glyph geometry as drawing.
  `run_command` closes the Lectern container before executing the command.
  Other server-allowed click events use the standard screen handler; the
  server-disallowed `open_file` event remains rejected.

Local sources under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/inventory/LecternScreen.java`,
`BookViewScreen.java`, `PageButton.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/item/component/WrittenBookContent.java`,
and `WritableBookContent.java` are authoritative.

## Ownership and frozen interfaces

- `bbb-world::client::book` owns conversion from an effective item stack to
  `BookViewPageState` and exposes one owned Lectern view containing container
  id, monotonic open-container revision, synchronized page, page list, and
  `mayBuild`. Standalone and Lectern book access must delegate to that same
  conversion rather than decode exact Component NBT independently.
- `bbb-world::InventoryState` owns a serialized, defaulted
  `open_container_revision: u64`. It advances on accepted open-container
  replacement, nonzero set-content, set-slot, and set-data mutations and on
  close. No wall clock, random hash, or renderer state participates.
- `bbb-native::runtime::book_screen` remains the sole world-to-render book
  projection owner. It projects standalone view/edit/sign or Lectern view into
  the existing `HudBookScreen`, reusing localization, rich runs, hover
  tooltips, current-session font resources, and text-filter selection.
- `bbb-renderer::HudBookMode` gains `Lectern`; `HudBookScreen` gains the bounded
  Take Book label and `mayBuild` flag. Lectern shares view-page drawing,
  wrapping, hover, and hit geometry with standalone view mode, while its footer
  shape differs. The renderer owns only presentation and returns revision-safe
  source-run targets.
- `bbb-native` retains existing inventory layout geometry as the sole mouse
  owner for page and footer controls and the existing container-button/close
  queue helpers as the sole command owner. Rich page targets enter before the
  generic Lectern mouse path. Page keys delegate to the same container-button
  helper.
- A rich click target is accepted only when its mode, open-container revision
  or standalone generation, synchronized page, resource generation/load state,
  filtering choice, and source run still match current canonical state.
  `run_command` queues ContainerClose before the chat command.
- The old `HudInventoryScreen` Lectern background, approximate ASCII wrapping,
  flattened page labels, and unconditional page sprites are removed. The input
  layout enum may remain because it owns hit geometry, not pixels.

## Fallback rules

1. A missing or unsupported slot-zero item produces `EMPTY_ACCESS`: the book
   screen, `Page 1 of 1`, footer, and no page controls remain visible.
2. Written pages prefer exact Component NBT; malformed or absent exact NBT
   falls back to the decoded plain page aligned at the same index.
3. Writable raw and filtered pages remain literal Components. A missing
   filtered alternative falls back to its raw page.
4. A missing current-session custom font uses the existing default-font
   fallback for both wrapping and hit testing; it does not change source-run
   identity.
5. Missing book/page resources preserve the existing book surface fallback and
   reject stale hit targets rather than executing an action from old pixels.

## Non-goals

- This slice does not change packet decoding, menu ids, item component codecs,
  server page authority, take-book inventory prediction, or generic container
  slot clicking.
- It does not change standalone writable-book editing/signing, chat click-event
  semantics, URL/dialog confirmation policy, or resource-pack font loading.
- It does not add book page search, accessibility narration infrastructure,
  an in-game text-filter option, or any other configuration UI.
- It does not close Beacon, Smithing, trial-spawner, or other presentation
  gaps.

## Serial stages and verification

1. **Canonical Lectern access and revision.** Share item-to-page conversion,
   add the monotonic open-container revision and owned Lectern view, and cover
   exact/raw/filtered/empty/page/revision behavior. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-world book` and
   `scripts/cargo-dev.sh test -p bbb-world inventory`.
2. **Native projection and input closure.** Project Lectern through
   `HudBookScreen`, delete the approximate inventory renderer path, accept
   revision-safe rich targets, preserve existing mouse button routing, add
   PageUp/PageDown, and enforce close-before-command ordering. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native book`,
   `scripts/cargo-dev.sh test -p bbb-native lectern`, and
   `scripts/cargo-dev.sh test -p bbb-native component_click`.
3. **Renderer and closeout.** Add Lectern mode drawing/sanitization/hit
   behavior, exact footer/page-control tests, and deterministic offscreen
   pixels. Remove the ledger item, archive this contract, and append the
   concise milestone index. Run
   `scripts/cargo-dev.sh test -p bbb-renderer book` plus the exact Lectern
   offscreen test, formatting, docs lint, diff-check, and full diff review,
   then commit clean and run the full gate.

The final full gate is the engineering checkpoint. Existing container-button
and close packet owners are retained, so semantic queue-order tests are the
interaction acceptance surface; no new live playable scenario is required.

## Failure localization and safe stops

World failures compare effective component choice, exact-NBT fallback, filter
selection, synchronized page, `mayBuild`, and revision advancement before
native projection. Projection failures compare localization, source runs,
interaction spans, filtering choice, and revision token. Input failures compare
hit owner, container id/button id, lack of local page mutation, then queued
packet order. Renderer failures compare mode sanitization, resident-font wrap,
button visibility/hover/footer geometry, command order, and finally pixels.

The clean contract commit and each buildable stage commit are safe stops. Any
gate failure is fixed forward before another slice; history is not rewritten.
