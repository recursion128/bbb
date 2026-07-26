# Remote-client crafting result prediction closure

## Goal and completion contract

Match the Vanilla 26.1 remote client's ownership of the Inventory and Crafting
menu result slots. Server `ContainerSetContent` / `ContainerSetSlot` packets own
the result stack. A local result click may predict only the client-visible
`ResultSlot.onTake` effects: consume one item from each occupied positioned input
slot and apply each input item's default crafting remainder. Primary pickup and
a fully transferred quick-move leave the result slot empty until the server
sends a correction; a capacity-limited transfer retains only its unmoved result
count.

Completion means Inventory menu input clicks never synthesize or clear result
slot `0` from RecipeBookAdd displays; Inventory and Crafting result pickup and
quick-move preserve the exact component-rich server result while predicting one
take; recipe-specific remainder recipes use the same item-default client
prediction as every other recipe; unknown item-default data remains a
conservative server-authoritative fallback; the obsolete recipe-specific
remainder membership catalog and publication path are removed; focused world,
native-input, item-model, and generator tests cover the new boundary; the
finished unsupported-feature item is removed; relevant packet coverage evidence
is refreshed without claiming unrelated specialized menu behavior; and the full
plus playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-world` owns server-supplied result-slot state, click prediction,
  positioned input consumption, default crafting remainders, inventory insertion
  for incompatible remainders, changed-slot hashes, and correction handling.
- `bbb-item-model`, `bbb-native` resource reload, and generator scripts stop
  extracting and publishing recipe-specific remainder input membership because
  the remote client does not consult it.
- `bbb-native` input keeps using the world click transaction and must queue a
  valid click when exact client prediction is available.

This slice does not implement recipe deserialization or local recipe matching;
server recipe evaluation; server-side repeated shift-crafting; creative
inventory; RecipeToast, focus, narration, or automatic RecipeBookSeenRecipe;
other specialized result-slot semantics; or a local dropped-item entity when an
incompatible remainder cannot fit the visible inventory. The last case still
sends the click and predicts the container slots exactly; the server owns the
eventual entity and authoritative correction.

## Vanilla and integration contract

`InventoryMenu.slotsChanged` calls
`CraftingMenu.slotChangedCraftingGrid` only when its owner level is a
`ServerLevel`. `CraftingMenu.slotChangedCraftingGrid` performs server recipe
lookup and sends `ClientboundContainerSetSlotPacket`; therefore neither
RecipeBookAdd displays nor client input mutation may derive the result stack.
The same authority boundary applies to the crafting-table result container.

`ResultSlot.getRemainingItems` asks the selected recipe for remainders only on a
`ServerLevel`. On the client it always calls
`CraftingRecipe.defaultCraftingReminder`, which reads the default
`ItemStackTemplate` from each input item's prototype. `ResultSlot.onTake` uses
`CraftingInput.Positioned`: it consumes exactly one item only from occupied
slots in the positioned rectangle, installs or merges a default remainder, and
otherwise tries the player inventory before dropping it.

`AbstractContainerMenu.doClick` repeats QUICK_MOVE while the source slot still
contains the same item. On a remote client, the first result transfer empties
the result container and `slotsChanged` does not recompute it, so the loop stops
after one take. The server may independently recompute and repeat while handling
the packet; later content/slot packets reconcile the client. Consequently bbb
must not restore the result template or use input counts as a local
`max_crafts`.

Unknown default-remainder metadata cannot reproduce the client item prototype,
so bbb keeps the existing fail-closed click guard until resource publication is
complete. Once known, absence from the default-remainder map means the normal
empty remainder. Recipe-specific recipe classes such as banner duplication and
book cloning do not alter the remote-client branch and must not block prediction.

Authoritative local sources are
`net/minecraft/world/inventory/InventoryMenu.java`,
`CraftingMenu.java`, `ResultSlot.java`, and `AbstractContainerMenu.java`;
`net/minecraft/world/item/crafting/CraftingRecipe.java`;
`net/minecraft/client/multiplayer/ClientLevel.java`; and
`net/minecraft/client/multiplayer/ClientRecipeContainer.java` under the local
Vanilla 26.1 source tree.

## Serial stages and verification

1. Remove Inventory menu RecipeBookAdd result synthesis and cover server-owned,
   component-rich result preservation across ordinary input pickup, placement,
   and quick-move. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-world inventory_menu_crafting_result`.
2. Make Inventory and Crafting result pickup/quick-move consume one positioned
   input set, use only default item remainders, retain only an unmoved result
   remainder, and keep unknown-default metadata fail-closed. Cover merge,
   inventory insertion, full inventory, component-rich result, recipe-specific
   inputs, and server correction. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-world crafting_result` and
   `scripts/cargo-dev.sh test -p bbb-native crafting_result`.
3. Remove recipe-specific remainder extraction, generated constants, item-model
   runtime fields, reload snapshots, world publications, and diagnostics.
   Regenerate the checked-in membership catalog and update generator fixtures.
   Focused tests:
   `scripts/vanilla-source-tools.sh python -m unittest
   scripts.tests.test_item_membership_catalog_extractor` and
   `scripts/cargo-dev.sh test -p bbb-item-model item_membership_catalog`.
   Because this stage changes generators, run `scripts/cargo-dev.sh gate` at its
   milestone commit.
4. Refresh the packet ledger only for dimensions fully evidenced by this slice,
   remove the completed unsupported item, archive this contract in the milestone
   index, review the integrated diff, then run `scripts/cargo-dev.sh gate`
   followed by `scripts/playable-gate.sh auto`.

The fully gated `267b9274e` is the pre-start checkpoint. Each stage is a clean,
buildable commit. World test failures stop before catalog removal; generator or
publication failures stop before coverage closeout; packet-ledger or playable
failures are fixed forward without rewriting history.
