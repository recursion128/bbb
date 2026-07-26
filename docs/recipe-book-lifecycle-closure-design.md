# Recipe-book lifecycle coverage closure

## Goal and completion contract

Close the remaining runtime and test coverage for
`ClientboundRecipeBookAddPacket` and the command/state/test coverage for
`ServerboundRecipeBookSeenRecipePacket`. A notified recipe must enter the same
ordered five-slot toast owner as advancement notifications, while a highlighted
recipe must remain highlighted until one of its fitting displays first becomes
visible in a native recipe button. That visibility edge removes the local
highlight and queues one `RecipeBookSeenRecipe` command in Vanilla button and
recipe order.

Completion means notification entries retain packet order and coalesce into
Vanilla's singleton recipe toast; advancement and recipe toasts share FIFO slot
admission, transition sounds, 600 ms slide animation, and five slots; the recipe
toast uses its exact background, localized text, category/result items, 5000 ms
change-relative lifetime, and multi-recipe cycling; visible highlights start
the matching tab/button animation, send exactly once, and retry without local
removal if the bounded net command channel is full; add/replace/remove,
menu-open, filter/search/tab/page, registry-reload, level-reset, and session
cleanup paths have focused coverage; recipe buttons and page/filter controls
provide the Vanilla hover tooltips and narration; the two packet rows are
complete; the finished TODOs are removed; and the full plus playable gates pass
at the milestone head.

## Scope, ownership, and non-goals

- `bbb-world` owns known recipe displays, highlights, settings, atomic packet
  application, local highlight removal, and one bounded, globally sequenced
  client-toast event handoff. Recipe notification events snapshot the first
  resolved crafting-station and result stacks at packet application time.
- `bbb-native` owns current recipe-book selection/search/page state, detection
  of newly visible fitting displays, successful command enqueue before local
  highlight removal, tab/button animation state, toast coalescing and timing,
  item/text localization projections, tooltip selection, and narration frames.
- `bbb-renderer` owns the recipe-toast sprite, item/text submissions, clipping,
  right-edge/slot layout, recipe-button animation geometry, and tooltip
  geometry. Backend handles remain outside canonical world state.
- `bbb-net` and `bbb-protocol` retain the existing typed seen-recipe command and
  encoder; this slice changes only the queue helper's success contract and its
  automatic caller.

This slice does not implement creative-inventory add/drop behavior, an in-game
notification-time option, recipe synthesis, authoritative crafting results, or
new keyboard navigation. Vanilla
`CreativeModeInventoryScreen` does not instantiate a `RecipeBookComponent`; its
creative slot interactions remain the separate `SetCreativeModeSlot` TODO.
Vanilla `RecipeBookComponent.setFocused` is a no-op and its inner recipe, tab,
filter, and page buttons are not screen focus children. Native therefore keeps
the existing clickable search-box focus and hover-driven inner-button
narration instead of inventing a keyboard-focus graph.

## Vanilla and integration contract

`ClientPacketListener.handleRecipeBookAdd` clears the book first for a replace
packet, then processes entries in wire order: add the display, add its highlight
when requested, and call `RecipeToast.addOrUpdate` for every notification.
`handleRecipeBookRemove` removes both the known display and its highlight.
Notification presentation is independent of later recipe removal because
`RecipeToast.addOrUpdate` resolves and stores the crafting-station/result
`ItemStack` pair immediately.

The world handoff uses one monotonic sequence across advancement and recipe
toast events so native FIFO order cannot change when both packet families occur
in one drain. It retains a fixed recent window and records overtaken events.
Native consumes each event once per client-level generation. Advancement
events create independent one-slot instances. All recipe events find the one
queued or still-visible recipe-toast token and append to it; the first creates
that token at its sequence position. Appending resets its change-relative
lifetime on the next update, including while it is hiding. Login, Respawn,
reconfiguration, and disconnect clear both the canonical handoff and native
queued/visible instances.

The shared toast owner matches `ToastManager`: five vertical 32-pixel slots,
FIFO admission, 600 ms squared show/hide movement with reversal from the
current visible portion, and at most one `ui.toast.in` or `ui.toast.out` sound
per update pass. Challenge admission keeps its existing once-per-pass sound.
Recipe toast visibility lasts until fully-visible time since the latest append
reaches 5000 ms. Its displayed pair is
`fullyVisibleMs / max(1, 5000 / itemCount) % itemCount`. It renders
`minecraft:toast/recipe`, localized `recipe.toast.title` at `(30, 7)` with
ARGB `0xFF500050`, localized `recipe.toast.description` at `(30, 18)` in black,
the crafting-station item at Vanilla's `0.6` scale from `(3, 3)`, and the
result fake item at `(8, 8)`.

Recipe button visibility follows `RecipeBookPage.updateButtonsForPage`: filter
the selected tab's fitting collections by search and craftability, select the
current 20-collection page, then inspect each collection's selected recipes in
stored order. `RecipeButton.init` removes every highlighted fitting recipe in
that visible collection and starts one 15-GUI-tick button animation.
`RecipeBookComponent.updateTabs` starts a separate 15-tick animation for every
visible category containing a highlighted selected recipe before current-page
buttons remove their highlights. Closing the book pauses render-owned
animation time. Nonselected tabs and later pages retain highlights until their
buttons become visible.

For each newly visible highlighted id, native first attempts the bounded
`RecipeBookSeenRecipe` enqueue. Only success calls the world-owned guarded
highlight removal, so an enqueue failure remains highlighted and retries on a
later simulation step. Removal makes future visibility passes no-ops and gives
exactly-once behavior without a second sent-id authority.

When no overlay is open, a hovered recipe button shows the current cycled
result's ordinary item tooltip plus localized
`gui.recipebook.moreRecipes` when the collection has multiple selected
recipes. Page arrows and the filter control retain their Vanilla localized
tooltips. Recipe narration uses `narration.recipe`, the hovered button's current
item name, `narration.button.usage.hovered`, and
`narration.recipe.usage.more` for a multi-recipe collection. A focused search
box has Vanilla terminal narration priority over hovered widgets; otherwise
the first active hovered widget in RecipeBook's narration order wins.

Authoritative local sources are
`net/minecraft/client/multiplayer/ClientPacketListener.java`,
`net/minecraft/client/player/LocalPlayer.java`,
`net/minecraft/client/gui/screens/recipebook/RecipeBookComponent.java`,
`RecipeBookPage.java`, `RecipeButton.java`, `RecipeBookTabButton.java`,
`net/minecraft/client/gui/components/toasts/RecipeToast.java`,
`ToastManager.java`, and
`net/minecraft/client/gui/screens/inventory/CreativeModeInventoryScreen.java`
in the local Vanilla 26.1 source tree.

## Serial stages and verification

1. Replace the advancement-specific canonical handoff with a bounded,
   globally ordered client-toast handoff and migrate the existing advancement
   runtime to consume it without behavior change. Focused tests cover global
   sequence/order, bounds, lag accounting, reset behavior, repeated completed
   advancements, and unchanged advancement FIFO/animation/sounds. This
   architecture-state milestone gets a full gate.
2. Add recipe event snapshots, singleton coalescing, change-relative lifetime
   and cycling, renderer/background/item/text projection, shared slot
   competition, and ordered sounds. Focused tests cover every recipe display
   family, empty first-stack fallback, multiple notification packets,
   queued/visible/hiding coalescing, advancement interleaving, five-slot
   saturation, exact timing/layout/colors, resource reload, hide-GUI, and
   session cleanup. This cross-runtime presentation milestone gets a full gate.
3. Add automatic visible-highlight handling, guarded enqueue/removal,
   per-tab/per-button animations, hover tooltips, and recipe-book narration.
   Focused tests cover open/closed books, selected/nonselected tabs, first and
   later pages, search/filter changes, multi-recipe order, command saturation
   retry, replace/remove, menu replacement, overlay suppression, focused
   search priority, and exactly-once send. This command/state milestone gets a
   full gate.
4. Mark both packet rows complete, remove the finished RecipeBook TODOs while
   leaving the separate creative-inventory TODO intact, append the milestone
   index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `2a2b4133b` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Ordered-handoff failures stop before recipe presentation;
toast coalescing or renderer failures stop before automatic seen-recipe
mutation; command/state failures stop before ledger closeout. Failures are
fixed forward without rewriting history.
