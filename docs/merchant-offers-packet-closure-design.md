# Merchant-offers packet coverage closure

## Goal and completion contract

Close the remaining decode, canonical-apply, runtime-side-effect, and test
coverage cells for `ClientboundMerchantOffersPacket`. Every `ItemCost` exact
component predicate must survive wire decode as typed component data, matching
merchant state must remain gated by the active `MerchantMenu`, and the native
screen must consume the complete offer state through the existing item,
tooltip, widget, and container-interaction owners.

Completion means each cost retains its holder, positive count, ordered exact
component values, and component-aware display stack; the sell stack retains
its complete patch; malformed counts, holder ids, component ids/values, empty
sell stacks, oversized collections, and trailing bytes fail without partial
state; matching-container packets replace offers and merchant metadata while
mismatched or non-merchant packets do nothing; row buttons use the vanilla
normal/highlighted widget sprites and seven-row viewport; modified cost A,
optional cost B, result, stock arrows, selected stock overlay, scroller,
current and future XP bars, discount strikethrough, stack decorations, item
tooltips, and the restock/deprecated tooltip are presented at vanilla
coordinates; selection sends one `ServerboundSelectTradePacket` after the
existing conservative local autofill; all five packet coverage dimensions are
`complete`; the finished `MerchantMenu` TODO is removed; and the full plus
playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns exact packet/offer/item-cost wire order, typed exact
  component values, strict bounds, holder validation, non-empty result stacks,
  and terminal-payload rejection. The existing data-component descriptor table
  remains the sole component value decoder.
- `bbb-world` owns active-menu/container-id authority, serializable offer and
  merchant metadata, selected offer/scroll state, component-aware cost display
  stacks, conservative local autofill/result prediction, and command ordering.
- `bbb-native` owns row hit testing and scroll/drag interaction, screen-local
  hover/focus projection, item-model/decorations/tooltips, row widgets, stock
  overlays, discount mark, and current/future XP projection.
- `bbb-renderer` owns only the shared widget, item, tooltip, fill, and villager
  sprite submissions. New merchant-specific draw paths are limited to missing
  enum/resource wiring for vanilla sprites already owned by the HUD renderer.

This slice does not implement server-side trading, synthesize an acknowledgement
that vanilla does not have, or make unsupported local item-component prediction
authoritative. Exact predicates are decoded and displayed. Local payment
autofill/result prediction may remain conservative when the client cannot prove
the effective component match; normal server container updates remain the
correction path. It does not generalize keyboard navigation across every
inventory menu, add creative inventory UI, change the shared tooltip content
model, or close unrelated menu TODOs.

## Vanilla, wire, and integration contract

`ClientboundMerchantOffersPacket` writes container id, a VarInt-sized
`MerchantOffers` collection, villager level and XP as VarInts, then
`showProgress` and `canRestock` booleans. `MerchantOffer.STREAM_CODEC` writes
cost A, non-empty sell `ItemStack`, optional cost B, out-of-stock boolean, then
uses, max uses, XP, special price difference, price multiplier, and demand.
`ItemCost.STREAM_CODEC` writes item holder, count, and
`DataComponentExactPredicate`; its display stack is the item/count plus that
predicate as a component patch. The existing bbb collection and component-entry
limits remain stricter defensive caps than vanilla's effectively unbounded
collection codec.

`ClientPacketListener.handleMerchantOffers` mutates state only when the packet
container id equals the current menu id and that menu is a `MerchantMenu`; it
replaces offers and sets villager XP, level, progress visibility, and restock
capability together. The local selection and scroll offset reset to zero for a
replacement packet. Row selection sets the menu selection hint, tries to move
matching inventory stacks into the two payment slots, and then sends exactly
one select-trade packet. Component predicates use exact value equality against
the effective stack; bbb keeps the server-authoritative fallback whenever its
retained item defaults cannot prove that equality.

`MerchantScreen` is 276 by 166. It creates seven 88-by-20 plain buttons at
`(5, 18 + row * 20)`, shows offers from `scrollOff`, and highlights the hovered
or focused button with the shared widget sprite. Costs/results render at x
10/40/73 and y `19 + row * 20`; arrows render at x 60 and y
`22 + row * 20`. A modified first cost renders the base count at x 10, the
modified count at x 24, and a 9-by-2 discount strike at `(17, 31 + row * 20)`.
Hovered x bands `<25`, `35..55`, and `>70` expose cost A, cost B, and result
tooltips. The selected out-of-stock overlay is at `(182, 35)`; when hovered at
`(186, 35, 22, 21)` and restocking is possible it shows
`merchant.deprecated`.

The XP background is 102 by 5 at `(136, 16)` for levels below five. Current XP
uses thresholds `[0, 10, 70, 150, 250]`; the selected offer contributes its XP
as a second result segment, clamped to the remaining width. The scroller uses a
six-by-27 sprite at x 94 and y 18 plus the exact stepped offset over a 139-pixel
track, with a disabled sprite for at most seven offers. Existing centered-screen
offsets apply after these local coordinates.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundMerchantOffersPacket.java`,
`net/minecraft/world/item/trading/ItemCost.java`, `MerchantOffer.java`,
`MerchantOffers.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java`,
`net/minecraft/world/inventory/MerchantMenu.java`, and
`net/minecraft/client/gui/screens/inventory/MerchantScreen.java` in the local
Vanilla 26.1 source tree.

## Serial stages and verification

1. Decode exact predicate values through the shared component descriptor table,
   retain a component-aware cost display patch, and carry it through world
   merchant state without weakening menu authority. Focused tests cover wire
   order, multiple typed predicate values, invalid/truncated/oversized/trailing
   input, matching versus mismatched/non-merchant menus, replacement reset, and
   conservative component-predicate autofill/result behavior. Because this
   changes protocol decode, the stage gets a full gate.
2. Complete native merchant interaction/presentation: row widget
   normal/highlighted state, selected offer continuity, current/future XP,
   component-aware cost icons, count/durability decorations, discount strike,
   all three item tooltip targets, and deprecated tooltip. Focused tests cover
   screen projection, seven-row scroll windows, hover bands, stock/restock
   behavior, selected offer XP, item component/name/decoration retention,
   renderer sprite mapping, and one-command selection ordering. This
   cross-runtime presentation milestone gets a full gate.
3. Mark the packet row complete, remove the finished merchant TODO, append the
   milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `49ee572fe` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Decode or canonical-state failures stop before native
presentation; interaction or projection failures stop before coverage closeout.
Failures are fixed forward without rewriting history.
