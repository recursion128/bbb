# Place-ghost-recipe packet coverage closure

Archived after the packet reached complete coverage across every ledger dimension.

## Goal and completion contract

Close the remaining canonical-apply and runtime-side-effect coverage cells for
`ClientboundPlaceGhostRecipePacket`. Every Vanilla 26.1 `RecipeDisplay` must
retain its structured result display, and every built-in `SlotDisplay` must
resolve through the existing world-owned registry, item-default, component,
fuel, potion, dye, and trim context before the ghost recipe becomes visible.

Completion means all five recipe-display wire variants are typed; the active
Inventory, Crafting, Furnace, Blast Furnace, and Smoker screen owns an immutable
candidate snapshot with exact result/input slot placement; invalid holders,
resolution failures, and mismatched containers preserve the previous screen
state; replacing the screen clears that state; the HUD cycles component-rich
candidates, renders Vanilla's result/input fills and decorations, and exposes
the selected ghost item tooltip while the recipe book is visible; all five
packet-coverage dimensions are `complete`; the finished unsupported-feature
item is removed; and the full plus playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the container-id prefix, the five fixed
  `RecipeDisplay` wire layouts, all eleven bounded recursive `SlotDisplay`
  layouts, exact raw payload retention, fixed component-type validation, and
  trailing-byte rejection.
- `bbb-world` owns dynamic holder validation, current registry/tag/fuel/item
  component resolution, immutable candidate snapshots, container/menu gating,
  crafting-grid placement, furnace fuel-slot gating, counters, and screen
  lifetime.
- `bbb-native` owns candidate cycling, component-aware item icons, pre/post
  fills, result decorations and enlarged result fill, hovered ghost tooltip,
  and reverse guards for unsupported or replaced screens.

This slice does not extend RecipeBookAdd/Remove toasts, overlay hover/focus or
narration; implement container-0 recipe matching/remainders; add creative
inventory behavior; or introduce an in-game configuration surface. Recipe-book
overlay entries may retain their existing narrow resolver until their separate
presentation item is selected, but PlaceGhostRecipe must not call that resolver
or create another SlotDisplay implementation.

## Vanilla, wire, and integration contract

`ClientboundPlaceGhostRecipePacket.STREAM_CODEC` writes the VarInt container id
followed by `RecipeDisplay.STREAM_CODEC`.
`ClientPacketListener.handlePlaceRecipe` compares the packet id with
`player.containerMenu.containerId` and calls `RecipeUpdateListener` only for the
active matching screen. The recipe component clears its old `GhostSlots`, builds
`SlotDisplayContext.fromLevel`, and resolves every assigned display immediately
with `resolveForStacks`; empty candidate lists do not create entries. Therefore
tag, fuel, registry, or item-default changes after packet handling must not
retroactively alter an existing ghost entry.

All five `RecipeDisplay` values expose `result()` and `craftingStation()`.
Shapeless and shaped displays additionally own ingredients; furnace owns
ingredient, fuel, duration, and experience; stonecutter owns input; smithing
owns template, base, and addition. Crafting screens always assign the generic
result to slot `0`, then assign only shaped/shapeless inputs. Inventory uses a
2x2 grid and Crafting uses 3x3. Shaped placement follows
`PlaceRecipeHelper.placeRecipe` exactly, including its narrow-dimension
centering. Furnace-family screens always assign the generic result to slot `2`;
only a furnace display assigns ingredient slot `0`, and it assigns fuel slot
`1` only when that real slot is empty at packet receipt.

`GhostSlots` selects `items[currentIndex % items.size()]`, draws the
`0x30ff0000` pre-item fill (24x24 around a big crafting-table result, otherwise
16x16), the fake item, the `0x30ffffff` post-item fill, and decorations only for
result slots. While the recipe book is visible, its tooltip extraction runs
after the ordinary container tooltip and therefore the selected ghost item wins
for a shared hovered slot. A newly installed screen owns a new empty
`GhostSlots`; bbb's `replace_screen` must provide the same lifetime boundary.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundPlaceGhostRecipePacket.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java`,
`net/minecraft/world/item/crafting/display/RecipeDisplay.java` and its five
records, `SlotDisplay.java`, `SlotDisplayContext.java`,
`net/minecraft/client/gui/screens/recipebook/GhostSlots.java`,
`RecipeBookComponent.java`, `CraftingRecipeBookComponent.java`,
`FurnaceRecipeBookComponent.java`, and
`net/minecraft/recipebook/PlaceRecipeHelper.java` under the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Type the stonecutter and smithing recipe-display bodies, expose one shared
   result/all-slot-display traversal, and add full five-variant plus eleven-
   semantic PlaceGhostRecipe decode tests. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-protocol client_features`. Because this
   changes protocol decode, the stage gets a full gate.
2. Move PlaceGhostRecipe resolution and placement into world canonical state,
   validate all dynamic holders atomically, snapshot candidate vectors at
   packet receipt, capture furnace fuel emptiness, and clear state on screen
   replacement. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-world place_ghost_recipe` plus existing
   SlotDisplay resolver tests. This state-ownership checkpoint is a clean
   buildable commit.
3. Make native presentation consume only the canonical snapshots; cover all
   eleven semantics, five eligible backgrounds, cycling, exact fills and
   decorations, tooltip precedence/visibility, and replaced/mismatched reverse
   guards. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native ghost_recipe`. This cross-runtime
   presentation milestone gets a full gate.
4. Mark the packet row complete, remove the finished unsupported item, archive
   this contract in the milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `839445b33` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Decoder failures stop before canonical migration; holder,
snapshot, or placement failures stop before native migration; presentation or
tooltip failures stop before coverage closeout. Failures are fixed forward
without rewriting history.
