# Book screens closure

Status: closed and archived after both packet coverage rows reached their final states.

## Goal and completion contract

Close the remaining applicable coverage cells for
`ClientboundOpenBookPacket` and `ServerboundEditBookPacket`, replacing the
literal read-only book projection and maintenance-only edit command with the
native Java 26.1 reading, editing, signing, and submission lifecycle.

Completion requires one world-owned book screen state that:

- opens a read-only view from the packet-selected held stack only when that
  stack exposes written- or writable-book content;
- retains both raw and filtered pages, exact trusted written-page Components,
  inherited style, font identity, and click/hover payloads instead of flattening
  written pages to strings;
- selects filtered or unfiltered presentation from the startup text-filtering
  option without deleting the other representation;
- opens a writable editor locally when ordinary use-item prediction succeeds
  for a held writable book, while the same input still queues the normal
  `ServerboundUseItemPacket`;
- initializes an empty writable book with one editable empty page, enforces 100
  pages, 1,024 UTF-16 units per page, and 14 wrapped lines in the editable
  region, and preserves the Vanilla PageUp/PageDown and append-page behavior;
- maps main hand to the selected hotbar slot and offhand to slot 40, revalidates
  the same held writable-book stack before submission, and never lets the
  maintenance command bypass the interactive screen's authority guard;
- handles Done by removing all trailing empty pages, queuing an unsigned edit,
  applying the matching local writable-content prediction only after queue
  acceptance, and closing the screen;
- handles Sign by preserving the current page list, editing a title of at most
  15 UTF-16 units, disabling Finalize for a blank title, returning unchanged
  pages and title to the editor on Cancel, and queuing the trimmed signed edit
  on Finalize;
- leaves signing/transmutation and filtered server output authoritative: the
  predicted unsigned patch is replaceable by later inventory packets and a
  signed book remains unchanged locally until the server's inventory update;
- discards unsaved edits on Escape, inventory-key close, level replacement,
  disconnect, held-stack loss/replacement, or replacement by another screen;
- renders the exact 192-by-192 book surface, page indicator, text region, page
  buttons, editor controls, sign controls, cursor/selection, and applicable
  hover tooltip/cursor state through native HUD primitives; and
- executes written-page click events only after revalidating a
  screen-generation, page, resource-generation, and run token against the
  current canonical page. `change_page` uses the one-based page payload,
  `run_command` follows the existing command-click path, and other supported
  click events use the existing shared platform/dialog/clipboard actions.

The final coverage ledger has no partial cell for either packet. Both book-screen
todos are removed in the same closeout commit, focused semantic tests cover the
lifecycle and authority edges above, and the final HEAD passes the full gate,
applicable playable gate, and P0 audit.

## Non-goals

- Do not implement an integrated server, server text filtering, account
  services, or client-authoritative signing/transmutation. Inventory packets
  remain the authority for the server result.
- Do not add in-game configuration. The text-filtering choice remains the
  command-line startup option already owned by native input state.
- Do not complete every ItemStack data component or general item-use behavior.
  This slice uses the generated writable-book item id plus the effective
  writable-book component needed by this exact interaction.
- Do not add the separate lectern menu rich-text/layout closure. The shared
  localized Component projection may be reused there later, but lectern menu
  authority and buttons remain independently deferred.
- Do not add custom-font glyph loading or bidi/unihex work. Preserve the font id
  in canonical style and use the existing renderer fallback when that font has
  no loaded glyph owner.
- Do not port the Java screen hierarchy, narration framework, or server-side
  `FilteredText` implementation. Preserve observable screen behavior through
  repo-native state and HUD types.
- Do not remove `net.edit_book`; it remains a maintenance transport surface.
  Its packet encoder limits stay unchanged, while the interactive path adds the
  held-item and screen-generation guards that a raw maintenance command cannot
  represent.

## Authoritative vanilla facts

- `GameProtocols.java` registers `CLIENTBOUND_OPEN_BOOK` with
  `ClientboundOpenBookPacket.STREAM_CODEC` and `SERVERBOUND_EDIT_BOOK` with
  `ServerboundEditBookPacket.STREAM_CODEC`.
- `ClientPacketListener.handleOpenBook` obtains the item in the packet-selected
  hand and opens `BookViewScreen` only when `BookAccess.fromItem` returns
  content. `BookAccess.fromItem` prefers written content, selects its filtered
  pages according to `Minecraft.isTextFilteringEnabled`, otherwise converts
  selected writable strings to literal Components.
- `BookViewScreen` uses a 192-by-192 book image at top offset 2, a 114-by-128
  page text region at offsets `(36, 30)`, page indicator offset `(148, 16)`,
  back/forward buttons at x offsets 43/116 and y offset 157, and a Done button
  below the image. It displays at most the 14 lines fitting the region.
- `BookViewScreen.handleClickEvent` interprets `change_page` as one-based,
  closes the server container before its special `run_command` path, and sends
  all other click events through the general screen click handler. Hovered text
  requests both tooltip and cursor effects.
- `WritableBookItem.use` calls `Player.openItemGui` and returns success.
  `MultiPlayerGameMode.useItem` invokes that local item use inside the sequence
  prediction before returning the matching `ServerboundUseItemPacket`, so a
  writable editor is a local prediction side effect rather than an OpenBook
  packet effect.
- `BookEditScreen` initializes from filtered or raw writable pages, adds one
  empty page when needed, limits a page to 1,024 UTF-16 units and `126 / 9`
  wrapped lines, appends a new empty page from Forward until 100 pages, and
  supports PageUp/PageDown.
- Editor Done removes every trailing empty page, writes pass-through writable
  content into the local stack, maps main hand to the selected inventory slot
  and offhand to 40, sends an unsigned edit packet, and closes. A generic screen
  close does not call this save path.
- `BookSignScreen` keeps the editor's current page list, limits the title widget
  to 15 UTF-16 units, disables Finalize for a blank title, displays the local
  player as author, returns to the same editor on Cancel, and sends the trimmed
  title on Finalize or confirmation key.
- `ServerboundEditBookPacket` encodes a VarInt slot, at most 100 strings of at
  most 1,024 UTF-16 units, and an optional title of at most 32 UTF-16 units.
- `ServerGamePacketListenerImpl.handleEditBook` accepts only hotbar slots or
  slot 40 and asynchronously filters the title/pages. The unsigned branch
  updates only a stack carrying `WRITABLE_BOOK_CONTENT`. The signed branch
  transmutes that stack to a written book, removes writable content, installs a
  filtered title, the player's plain name, generation zero, literal Component
  pages, and `resolved = true`, then replaces the inventory slot.

Authoritative sources are under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/`:
`network/protocol/game/{GameProtocols,ClientboundOpenBookPacket,ServerboundEditBookPacket}.java`;
`client/multiplayer/{ClientPacketListener,MultiPlayerGameMode}.java`;
`client/gui/screens/inventory/{BookViewScreen,BookEditScreen,BookSignScreen}.java`;
`world/item/WritableBookItem.java`; and
`server/network/ServerGamePacketListenerImpl.java`.

## Ownership and frozen integration surface

### Protocol and item component boundary

- The existing `OpenBook`, `EditBook`, decoder, encoder, and packet limits remain
  the sole wire owners. This slice changes protocol code only if focused
  boundary tests expose missing evidence.
- `DataComponentPatchSummary` remains the decoded ItemStack owner. Written pages
  use `written_book_pages_nbt` and
  `written_book_page_filters_nbt` as exact trusted Component trees, while the
  plain summaries remain diagnostics/fallbacks. Writable raw and filtered
  strings remain paired by page index.
- A new narrow protocol helper constructs a literal `RichTextComponent` from a
  writable string without fabricating serialized server data. It does not
  create another general Component codec.

### Canonical world/UI state

- `crates/bbb-world/src/client/book.rs` becomes the semantic book-screen owner.
  `client/ui.rs` keeps only the `ClientUiState` field, replacement/return-stack
  integration, and facade calls needed by other screen families.
- `BookScreenState` becomes an enum with `View`, `Edit`, and `Sign` variants.
  Every variant carries one monotonically increasing screen generation and the
  originating hand. View pages retain raw and optional filtered
  `RichTextComponent`s; Edit owns mutable strings and page index; Sign owns the
  same pages plus mutable title and author display text.
- `WorldStore` exposes narrow open/query/page/edit/sign/close methods. Native
  code cannot mutate the enum fields directly. Page and title mutations enforce
  UTF-16 bounds at the world boundary; visual cursor/selection do not enter
  canonical state.
- `apply_open_book` continues to update the raw packet diagnostic and counter
  even when the held item has no book access. A successful open replaces the
  current screen through the existing single-screen policy.
- Local editor opening receives the startup filtering choice, checks the
  generated writable-book item id and effective writable component in the
  current hand, snapshots selected pages, and replaces the current screen.
- A submission snapshot contains generation, hand, resolved inventory slot,
  pages, and optional title. The world revalidates generation, mode, selected
  slot, item id, and effective writable component before returning it. A second
  world call commits the unsigned local writable-page prediction only after the
  native queue accepts the command; later inventory application replaces it
  normally. Signed submission has no local item mutation.
- Existing level/screen cleanup closes all three variants. A held-stack
  synchronization that no longer matches a writable editor/sign screen also
  closes it before further input can submit stale content.

### Localized Component projection and interaction tokens

- `crates/bbb-native/src/component_text.rs` owns one localized recursive
  Component traversal that produces rich runs preserving inherited
  `ComponentStyle`, click event, hover event, insertion, and font. It absorbs the
  existing localized traversal currently private to entity name tags; entity
  name tags consume the shared projection without changing their observable
  output.
- Resource localization occurs in native projection, not world or renderer.
  Renderer book runs retain enough source-run identity to map wrapped glyph
  positions back to canonical Component runs.
- `BookPageInteractionToken` carries world screen generation, zero-based page,
  resource generation/load state, and source-run index. Native stores only the
  currently published layout tokens. The simulation owner revalidates a token
  against the active View variant before dispatching its click event.
- `change_page` is handled against the active book. `run_command`, URL, dialog,
  clipboard, suggestion, and other supported events reuse the existing shared
  chat/dialog click-action paths and platform policy. Invalid, stale, unknown,
  or currently unsupported actions are inert. Hover events use the existing
  Component tooltip projection/fallback and never mutate world state.

### Native input, transport, and rendering

- `crates/bbb-native/src/input/book.rs` owns native-only page/title cursor and
  selection, drag selection, focus, vertical preference, scroll-free book hit
  testing, click dispatch, local editor opening, Done/Sign/Cancel/Finalize, and
  queue-failure handling. `input.rs` remains its facade and global ordering
  integration point.
- Ordinary right-click keeps the existing use-item command/sequence order. When
  the selected hand is a writable book, the same prediction step opens Edit and
  releases gameplay input; it does not suppress the use-item packet.
- `queue_edit_book_command` remains the one transport call. Interactive
  submission increments the existing command counter only on queue acceptance.
  Queue rejection retains the current editor/sign screen and makes no local
  inventory prediction, allowing retry instead of losing unsent work.
- `crates/bbb-renderer/src/hud/book.rs` owns book screen render data, exact
  geometry, styled wrapping, source-run hit spans, hover tooltip projection, and
  editor cursor/selection quads. `hud.rs` remains the facade/GPU integration
  point and does not gain another large inline feature block.
- The current `HudFontGlyphMap` and renderer implement one shared book text
  measurement interface so simulation hit testing and rendered wrapping use the
  same glyph advances. Existing missing-font fallback remains deterministic.
- The book surface participates in the existing single-screen suppression,
  background blur, cursor release, and immutable world/control publication
  ordering. Renderer types own presentation only; they do not retain editable
  canonical pages.

## Serial stages and verification

1. Add the world-owned rich View/Edit/Sign lifecycle and move the stable book
   state out of the oversized UI owner. Focused tests cover written/writable
   priority, raw/filtered pairing, empty-page initialization, bounds, page
   append, trailing trim, title rules, screen generation, held-slot authority,
   unsigned prediction, signed no-prediction, inventory correction, and level
   cleanup. Run
   `scripts/cargo-dev.sh test -p bbb-world book_screen` and
   `scripts/cargo-dev.sh test -p bbb-protocol edit_book`.
2. Add the shared localized Component projection, refactor entity name tags onto
   it, and add native local-use/editor/sign input plus guarded command
   submission. Focused tests cover the existing name-tag output, local opening
   without suppressing UseItem, UTF-16/line limits, cursor/selection, page keys,
   Done/Escape differences, Sign/Cancel/Finalize, queue failure, slot changes,
   and unsigned prediction. Run
   `scripts/cargo-dev.sh test -p bbb-native book_screen`,
   `scripts/cargo-dev.sh test -p bbb-native entity_name_tag`, and
   `scripts/cargo-dev.sh test -p bbb-net edit_book`.
3. Add the dedicated renderer book surface and revision-scoped interaction
   layout. Focused tests cover exact geometry, 14-line clipping, styled wrapping,
   filtered-page selection, source-run hit spans, hover tooltips,
   `change_page`, stale-token rejection, the shared click-action path, editor
   selection/cursor, and screen suppression. Run
   `scripts/cargo-dev.sh test -p bbb-renderer book_screen` and
   `scripts/cargo-dev.sh test -p bbb-native book_screen`.
4. Remove the completed todos, mark both packet rows complete with exact
   evidence, regenerate/check the packet ledger, append the milestone index,
   archive this contract, and run the final full gate,
   `scripts/playable-gate.sh live`, `scripts/playable-gate.sh auto`, and
   `scripts/audit-p0.sh`.

The fully gated `262533a41` commit is the pre-start checkpoint. Every stage ends
in a clean, buildable commit. Run a checkpoint full gate after stage 2 because
the canonical/transport authority boundary is then complete, or earlier if that
stage brings the cumulative engineering slice to four commits. Run the final
full gate after closeout. Failures are fixed forward without rewriting history.
The live playable gate may report missing external Java/server/EULA/display
prerequisites rather than expanding this slice into machine provisioning; the
fixture policy must still pass.

## Failure localization and safe stopping

- Component decode/projection failures belong to protocol/native focused tests;
  world state continues to retain exact trusted NBT and may present its existing
  plain diagnostic fallback without losing the source tree.
- World lifecycle or held-slot failures stop before native submission work; the
  prior read-only screen remains buildable and packet decode remains unchanged.
- Native queue failures never close the editor or predict inventory state.
  Renderer failures cannot mutate pages, inventory, or command queues.
- A failed full gate is fixed at the current tip. `batch-gate` is used only when
  the introducing stage cannot be identified from its focused tests.
- Safe stopping points are the clean commits after stages 1, 2, or 3. The slice
  is not complete and ledger/todos are not closed until stage 4 and the final
  full gate pass.
