# Creative inventory and SetCreativeModeSlot closure

## Goal and completion contract

Implement the native Vanilla 26.1 creative-inventory screen and close the
automatic `ServerboundSetCreativeModeSlotPacket` lifecycle. A player with
infinite materials opens the 195 by 136 item-picker menu instead of the normal
inventory, can browse the exact enabled creative tabs and generated item order,
search names, tooltips, and live item tags, scroll the 5 by 9 grid, manipulate
creative items and the real hotbar/inventory, clear slots through the trash
slot, and drop stacks subject to the client throttler. Losing infinite
materials replaces the screen with the ordinary inventory.

Completion means all fourteen built-in tabs, their ordering, visibility, icons,
backgrounds, tab sprites, search box, scroll thumb, item grid, inventory-tab
slot mapping, trash slot, hover tooltips, narration, carried stack, and hotbar
projection have focused coverage; catalog membership is generated from local
Vanilla sources; dynamic stacks come from the matching frozen registries and
item defaults; every automatic add/drop command passes Vanilla's local gates
and is retried after bounded-channel saturation without duplicating accepted
commands; server corrections update canonical and remote inventory state;
creative screens suppress `SetCursorItem`; hotbar increases set five ticks of
pop time; the three packet rows are complete; the finished TODOs are removed;
and the full plus playable gates pass at the milestone head.

## Scope, ownership, and integration surface

- `scripts/creative_mode_tab_catalog_extractor.py`,
  `scripts/generate-creative-mode-tab-catalog.py`, and their focused Python
  tests own the Tree-sitter extraction and deterministic Rust generation. The
  extractor consumes `CreativeModeTabs.java`, `CreativeModeTab.java`,
  `Items.java`, and `ItemIds.java`; it emits tab metadata, ordered static item
  references, visibility, and typed dynamic-family markers. It must reject
  unknown builder shapes, duplicate tab keys, missing accepted-item calls, and
  source drift rather than silently omit content.
- `bbb-world::creative_inventory` owns generated catalog facts, materialized
  item stacks, canonical creative-screen mode, inventory-menu remote slots,
  carried-stack suppression, hotbar pop time, and guarded local mutations. Its
  cross-crate surface is `CreativeInventoryTab`, `CreativeInventoryTabType`,
  `CreativeInventoryScreenState`, `CreativeInventorySnapshot`,
  `CreativeInventoryAction`, and narrow `WorldStore` query/apply methods. The
  generated facts live in
  `crates/bbb-world/src/creative_inventory/generated_26_1.rs`; callers do not
  redeclare tab or membership tables.
- `bbb-native::input::inventory::creative` owns selected tab, search editing,
  scroll position and dragging, hover/focus/narration selection, pointer and
  keyboard gesture interpretation, the 20-tick/1280-drop local throttler, and
  enqueue-before-commit retry state. It emits existing typed
  `NetCommand::SetCreativeModeSlot` values and applies a world action only after
  the bounded command queue accepts every packet required by that action.
- `bbb-native::runtime::creative_inventory` owns the immutable world-to-HUD
  projection, localized tab/search/tooltip text, and resource-generation
  rebuild. The general inventory runtime delegates creative projection instead
  of adding a parallel implementation to `runtime.rs`.
- `bbb-renderer::hud::creative_inventory` owns the three creative backgrounds,
  top/bottom selected/unselected tab sprites, scroller sprites, exact clipping
  and layer order, grid/hotbar/trash geometry, text caret/selection, tooltip
  geometry, and deterministic offscreen assertions. The existing
  `HudInventoryScreen` remains the common submission envelope and gains a
  creative-specific payload rather than a second screen submission API.
- `bbb-net` and `bbb-protocol` keep the existing typed packet and encoder. No
  manual control request becomes a second gameplay authority; the control
  surface continues to expose its explicit diagnostic command.

The screen-local selection state is keyed by client-level generation and the
materialized catalog revision `(registry, tags, enabled features, game-master
permission)`. Catalog rebuild replaces invalid selection with the first visible
category tab, clamps scroll, clears incompatible hover/drag state, and preserves
the search string only while search remains selected. Login, Respawn,
reconfiguration, disconnect, menu replacement, and resource reload clear
generation-owned interaction tokens. A failed catalog rebuild retains the
previous accepted immutable catalog and reports the failure; it never publishes
a partially rebuilt list.

This slice does not implement the creative inventory-tab player preview, because
that is a separate entity-in-UI render owner already tracked with a restart
criterion. Persisted `hotbar.nbt` load/save and its activator-key UI are also a
separate platform/options-file lifecycle: this slice renders the Vanilla empty
hotbar placeholders and keeps the current real nine-slot hotbar fully
interactive, but does not invent a second persistence owner. Tutorial pickup
presentation from `SetPlayerInventory` remains its existing separate TODO.

## Vanilla catalog and screen contract

`CreativeModeInventoryScreen` is constructed only while
`LocalPlayer.hasInfiniteMaterials()` and installs `ItemPickerMenu`. Its menu has
45 item-picker slots in five rows of nine at `(9 + column * 18,
18 + row * 18)` and the nine real hotbar slots at `(9 + column * 18, 112)`.
The screen uses `tab_items.png`, `tab_item_search.png`, or
`tab_inventory.png`; the 12 by 15 scroll thumb moves at x 175 over y 18 through
115 and is disabled when `ceil(items / 9) - 5` is nonpositive. Tab buttons are
26 by 32, spaced 27 pixels, and select one of the exact top/bottom,
selected/unselected sprites.

The generated catalog preserves the seven top and seven bottom tabs in Vanilla
order: building blocks, colored blocks, natural blocks, functional blocks,
redstone blocks, hotbar, search; then tools and utilities, combat, food and
drinks, ingredients, spawn eggs, operator blocks, inventory. Category, search,
hotbar, and inventory tab types retain their distinct layouts. Operator content
requires both game-master-block permission and the display-operator-tab option.
Every stack is filtered by its required enabled features.

Static `output.accept(Items.X)` membership and parent/search visibility comes
from the generated source facts. Runtime materialization owns only the typed
families that Vanilla derives from synchronized data: craftable firework
durations, instruments, potion items, suspicious-stew effects, ominous-bottle
amplifiers zero through four, maximum-level parent enchantment books plus
all-level search entries, preset paintings sorted by area then width, and
operator test-block/light variants. Stacks use the existing complete
`DataComponentPatchSummary`, frozen registry holder ids, default item component
profiles, and stable item protocol-id catalog. Search is the insertion-ordered,
type-plus-components de-duplicated union of visible tabs.

The search box is `(82, 6, 80, 9)`, accepts at most 50 characters, lowercases
ordinary queries against creative names and tooltip lines, and interprets a
leading `#` as a live item-tag query. Matching tag ids and tab identity append
their Vanilla extra tooltip lines. Selected-tab, search, and focused edit-box
narration use the same translated keys and precedence as the source screen.

## Input, command, and correction contract

Creative picker slots follow `slotClicked`: pickup installs a copy, right click
shrinks one, quick move grows to the maximum, clone produces the maximum stack,
number keys replace the target hotbar slot with a maximum stack, and throw drops
one or the full stack. A stack carrying `creative_slot_lock` cannot be taken.
Outside pickup becomes throw: left drops the carried stack and right drops one.
All picked or emitted items must still be enabled.

The inventory tab wraps the real inventory slots. Pickup and quick-move delegate
to the existing canonical inventory owner; throw sends the dropped stack and
the resulting real slot; trash clears the carried stack; shift-trash sends empty
updates for player slots 0 through 45 in ascending order. On non-inventory tabs,
every changed hotbar slot is sent with its Vanilla inventory slot number, and
quick-craft header two synchronizes all nine hotbar slots in ascending order.
Multi-packet actions are admitted atomically against the local bounded command
sink before canonical mutation so saturation cannot leave half a trash clear or
hotbar synchronization.

`MultiPlayerGameMode.handleCreativeModeItemAdd` sends only with infinite
materials and an enabled item. Drop also requires no open non-creative
`AbstractContainerScreen`, a nonempty enabled stack, and local throttler count
below 1280; accepted drops use slot `-1` and add 20 to the throttler. One count
is removed per client tick. Slot writes use player inventory slots 1 through 45
and nonempty counts no greater than the stack's effective maximum. The server
retains authority for creative ability, reach, game-master data, hotbar
selection, inventory mutation, and its own drop throttler.

`ClientPacketListener.handleContainerSetSlot` always applies container id zero
to the inventory menu. In a category/search creative screen it updates only the
inventory menu's remote slot and broadcasts changes, not the picker grid or
active creative view. A hotbar transition from empty or a smaller count to a
nonempty larger stack sets pop time to five. Matching nonzero menus continue to
use the existing active-menu gate. `handleSetCursorItem` keeps its current
noncreative behavior but does not replace carried state while any creative
screen is open. `handleSetPlayerInventory` remains the authoritative direct
inventory correction path; its tutorial side effect is outside this slice.

## Serial stages and verification

1. Add the Tree-sitter extractor/generator, generated catalog owner, typed
   dynamic markers, and deterministic reproduction checks. Focused tests cover
   all fourteen tab identities and positions, exact static acceptance count and
   order, parent/search visibility, every dynamic family, malformed/drifted
   source rejection, generated-file freshness, duplicate detection, and the
   no-parallel-facts invariant. Because this changes a source generator, run the
   full gate after the stage.
2. Materialize the catalog from the frozen item/registry/tag/feature/permission
   revision and add world creative-screen/remote-slot/pop-time state. Focused
   tests cover all dynamic component values and ordering, de-duplication,
   feature and operator gates, rebuild/rollback, creative open and ability-loss
   replacement, reset edges, `SetCursorItem` suppression, remote-slot sync, and
   five-tick hotbar pop time. Run the full gate at this canonical-state
   milestone.
3. Add native screen state and renderer projection for tabs, backgrounds,
   search, scrolling, item/hotbar/inventory/trash slots, clipping, tooltips, and
   narration. Focused tests cover exact geometry and layer order, all tab types,
   zero/one/many scroll rows, search/tag matching, selection preservation and
   invalidation, resource reload, small windows, GUI scale, hidden GUI, and
   deterministic offscreen pixels. Run the full gate at this cross-runtime
   presentation milestone.
4. Add all picker, inventory, hotbar, trash, outside-drop, clone, number-key,
   quick-craft, and throttle interactions through the existing typed command.
   Focused tests cover every button/click type, disabled and locked stacks,
   invalid slot/count, open-container and ability gates, exact packet slot/order,
   all-or-nothing saturation retry, local mutation after acceptance, throttler
   decay/limit, server correction, menu replacement, and connection reset. Run
   the full gate at this command/state milestone.
5. Mark `SERVERBOUND_SET_CREATIVE_MODE_SLOT`,
   `CLIENTBOUND_CONTAINER_SET_SLOT`, and `CLIENTBOUND_SET_CURSOR_ITEM`
   complete, remove their finished TODOs while preserving the separate hotbar
   persistence, entity-preview, and tutorial TODOs, archive this contract,
   append the milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `7efc67e1a` is the pre-start checkpoint. Every stage is a clean,
buildable commit. Generator/catalog failure stops before canonical screen state;
state/rebuild failure stops before renderer work; presentation failure stops
before automatic mutation; command/saturation failure stops before ledger
closeout. Failures are fixed forward without rewriting history.
