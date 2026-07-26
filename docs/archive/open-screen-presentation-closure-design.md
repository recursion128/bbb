# Open-screen packet coverage closure

## Goal and completion contract

Close the remaining canonical-apply and runtime-side-effect coverage cells for
`ClientboundOpenScreenPacket`. The trusted title `Component` must survive
decode and world storage as one exact validated value, and every generated
Vanilla 26.1 `MenuType` must select a native server-opened screen layout with
the title and player-inventory labels that its Vanilla screen actually draws.

Completion means `OpenScreen` and `ContainerState` own a
`RichTextComponent` rather than independently mutable plain/styled
projections, container content/data updates preserve that component exactly,
native title presentation localizes translatable content while retaining
inherited visual styles, centered titles are centered by the renderer's real
glyph metrics, `LecternScreen` continues to ignore the packet title, all 25
generated menu ids pass a reverse layout/presentation guard, both remaining
packet coverage cells are `complete`, the finished generic menu-layout/title
TODOs are removed or narrowed, and the full plus playable gates pass at the
milestone head.

## Scope and non-goals

- `bbb-protocol` owns the container id, generated menu registry id, exact
  trusted title Component, unknown-id rejection, and trailing-byte rejection.
- `bbb-world` owns the current server-opened `ContainerState`, exact title,
  menu id, retained content/data/offers, and replacement lifecycle.
- `bbb-native` owns generated-menu-to-layout routing, Component localization,
  Vanilla label placement, merchant title/level/trades composition, and HUD
  extraction.
- `bbb-renderer` continues to own glyph measurement, styled glyph drawing,
  clipping/sanitization, and centered-label placement through the existing
  scrolling/centering text contract.

This slice does not implement the creative inventory screen, mount-screen
title handling, recipe `SlotDisplay` semantics, rich Lectern page interaction,
or the remaining Beacon, Enchantment, Cartography, Loom, Lectern, and Smithing
menu-specific animations/previews/tooltips listed separately in the feature
ledger. It does not add an in-game configuration surface. Those gaps do not
prevent `MenuScreens.create` from constructing the matching screen and are
not packet-open lifecycle gaps.

## Vanilla, wire, and integration contract

`ClientboundOpenScreenPacket.STREAM_CODEC` writes
`ByteBufCodecs.CONTAINER_ID`, a `Registries.MENU` registry id, then
`ComponentSerialization.TRUSTED_STREAM_CODEC`. The protocol decoder keeps the
existing fixed 25-entry generated catalog check and changes only the title
owner to the shared `RichTextComponent` decoder. World apply moves that single
value into `ContainerState`; later set-content, set-data, set-slot, and
merchant-offers packets never reconstruct or flatten it.

`ClientPacketListener.handleOpenScreen` delegates directly to
`MenuScreens.create`. Vanilla's `MenuScreens` static registry contains all 25
`MenuType` values in the same order as `VANILLA_MENU_TYPES`; construction
replaces `player.containerMenu` and opens the corresponding screen. The native
layout route must therefore be total over that generated catalog. A test
iterates the catalog instead of naming only examples, so a future generated
menu addition cannot silently fall through to no screen.

Most registered screens inherit `AbstractContainerScreen.extractLabels`,
which draws the packet title and `Inventory.getDisplayName()` at
`titleLabelX/titleLabelY` and `inventoryLabelX/inventoryLabelY` with color
`0x404040` and no shadow. Dispenser/Crafter/Brewing/Furnace-family titles are
centered; Anvil, Crafting, Loom, Merchant, Smithing, Cartography, and
Stonecutter adjust the inherited coordinates; Beacon/Hopper/Shulker alter
screen height and therefore the inventory-label Y. `MerchantScreen` replaces
the base title label with its centered title or localized title-plus-level,
adds the localized trades label, and moves the inventory label to X 107.
`LecternScreen` extends `BookViewScreen`, ignores its constructor title, and
draws neither base label. Native uses the shared localized Component visitor
for the packet title and the existing item-runtime language table for fixed
labels; renderer-side centering uses the actual loaded glyph map.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundOpenScreenPacket.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java#handleOpenScreen`,
`net/minecraft/client/gui/screens/MenuScreens.java`,
`net/minecraft/client/gui/screens/inventory/AbstractContainerScreen.java`,
and the 21 registered screen classes adjacent to it in the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Decode the title as `RichTextComponent`, migrate `OpenScreen` and
   `ContainerState` to that exact canonical value, mechanically update
   callsites to use plain components, and prove rich NBT/style preservation
   across decode, world apply, content refresh, and serde. Focused tests:
   `bbb-protocol` open-screen packet tests and `bbb-world` open-screen/title
   tests. This protocol-decode stage ends with a full gate.
2. Add server-opened base labels, localized styled title projection, exact
   per-screen coordinates/centering, merchant composition, and a reverse test
   over all generated menu ids. Focused tests: `bbb-native` inventory layout
   and runtime HUD open-screen tests plus relevant renderer text-centering
   tests.
3. Mark the packet row complete, remove or narrow only the finished menu
   title/layout ledger entries, append the milestone index, review the
   integrated diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `5eae85962` is the pre-start checkpoint. Stage 1 and the final
milestone are clean, buildable commits. Decode/type migration failures stop
before native presentation; layout/catalog failures stop before coverage
closeout. Failures are fixed forward without rewriting history.
