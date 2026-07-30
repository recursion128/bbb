# Beacon Screen Presentation Engineering Contract

Status: completed on 2026-07-30.

## Goal and completion

Close the active `BeaconMenu` presentation gap with the exact observable
Vanilla 26.1 screen behavior:

- preserve the synchronized and screen-local primary/secondary selections,
  selected sprites, and tier gating already present;
- add exact disabled/selected/hovered/normal sprite precedence for every power
  button and active/hovered/normal precedence for confirm and cancel;
- replace the generic container title and inventory labels with the centered
  `Primary Power` and `Secondary Power` labels;
- project the localized effect, upgrade, confirm, and cancel tooltips at the
  cursor; and
- draw the five ordered example payment items embedded in the screen
  background.

Completion requires native semantic tests for disabled, selected, hovered,
active-confirm, labels, tooltip localization, upgrade visibility, missing
assets, and the example-item order; renderer command-order tests and
deterministic offscreen pixel evidence for disabled, selected, highlighted, and
normal button strata plus the example items; removal of the active ledger item;
archival of this contract; a concise milestone index entry; and a passing full
gate at the final head.

## Authoritative behavior

- `BeaconScreen` has a `230x219` image. It overrides `extractLabels` and draws
  only `block.minecraft.beacon.primary` centered at `(62,10)` and
  `block.minecraft.beacon.secondary` centered at `(169,10)`, both with Java
  color `-2039584` (`0xffe0e0e0`). It does not draw the menu title or the
  player-inventory label.
- Primary effect buttons cover tiers zero through two, are centered on `x=76`,
  start at `y=22`, use 22-pixel buttons with 24-pixel horizontal spacing, and
  advance by 25 pixels per tier. Secondary buttons are centered on `x=167` at
  `y=47` and include the tier-three effect plus an upgrade button.
- A power button is active exactly when `tier < levels`. Its base sprite
  precedence is disabled, selected, hovered-or-focused, then normal. Its
  18-by-18 effect icon starts two pixels inside the 22-by-22 base.
- The upgrade button is invisible until a primary selection exists. It then
  uses the current primary effect for its icon and localized tooltip with the
  literal ` II` suffix. A base power button's tooltip is its localized effect
  description.
- Confirm is at `(164,107)`, is active only when the payment slot is nonempty
  and a primary effect is selected, and uses `gui.done` as its tooltip. Cancel
  is at `(190,107)`, is always active, and uses `gui.cancel`. An active hovered
  action uses the highlighted base; a disabled confirm retains the disabled
  base even while hovered.
- `extractBackground` draws example `netherite_ingot`, `emerald`, `diamond`,
  `gold_ingot`, and `iron_ingot` stacks at `(20,109)`, `(41,109)`, `(63,109)`,
  `(86,109)`, and `(108,109)` in that order, without item decorations.

The authoritative local source is
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/inventory/BeaconScreen.java`;
effect-tier and menu-data behavior is confirmed against
`net/minecraft/world/level/block/entity/BeaconBlockEntity.java` and
`net/minecraft/world/inventory/BeaconMenu.java`. Generated
`VANILLA_BEACON_EFFECT_TIERS`, mob-effect metadata, and item metadata remain the
repo-native identity sources.

## Ownership and frozen interfaces

- `bbb-world` remains the canonical owner of the open container, payment slot,
  beacon levels, and server-confirmed primary/secondary effect data. It gains no
  cursor, hover, localization, or renderer state.
- `bbb-native` remains the sole world-to-HUD projection owner.
  `InventoryHudLocalState` continues to carry the screen-local unsent selection
  and cursor position. Beacon geometry stays screen-relative and uses the same
  effect-button layout helper as the existing projection.
- The native button projection derives one base sprite per visible button with
  the Vanilla precedence above, followed immediately by its icon. It adds no
  duplicate hover state or second selection owner.
- The native text projection emits exactly the two centered localized labels
  for Beacon and bypasses generic opened-container labels. The existing HUD
  font owner measures and draws them.
- The native tooltip projection returns one plain single-line
  `HudInventoryTooltip` anchored at the screen-relative cursor with
  `slot_id = u16::MAX` and no item tooltip style. Beacon control tooltips win
  over an overlapping inventory-slot tooltip.
- The five example items are ordinary `HudInventoryItem` fake-item
  projections, resolved through the current `NativeItemRuntime` and existing
  item/block-model residency paths. Their generated Vanilla resource
  identities are mapped through the current runtime; no handwritten runtime
  protocol-id table or renderer-specific item cache is added.
- `bbb-renderer` keeps the existing typed Beacon sprites, HUD item-model
  residency, font, tooltip, and ordered inventory-screen draw paths. This slice
  adds proof around those consumers, not a Beacon-specific renderer branch.

## Bounds and fallback rules

1. A disabled power button or confirm button never becomes highlighted because
   of hover. A selected active power button remains selected while hovered.
2. A missing or invalid local primary selection omits the upgrade button. An
   effect id without generated Vanilla metadata omits its icon and tooltip
   rather than fabricating a name, while retaining any valid button base
   geometry.
3. A missing item runtime omits all five example payment icons but preserves
   the complete background, buttons, labels, slots, and tooltips that can use
   fallback English text. An individual unresolved example item omits only
   that item and preserves the order of the remaining items.
4. Tooltips use current resource-pack translations when resident and the exact
   Vanilla English strings as fallback. The upgrade suffix remains literal
   ` II`.
5. Existing sprite upload failures retain the renderer's bounded missing-sprite
   behavior. This slice does not decode, upload, or retain a second copy of any
   Beacon or item texture.

## Non-goals

- This slice does not change Beacon packet decoding, container data authority,
  click hit-testing, menu-button command construction, confirm/cancel command
  routing, or container close behavior.
- It does not add focus traversal, narration, pointer-cursor selection, or an
  in-game configuration surface.
- It does not refactor the generic inventory projection, item-model runtime, or
  HUD command architecture and does not close other menu, HUD, mob-armor,
  fine-grained GPU-state, or sampler/mipmap gaps.

## Serial stages and verification

1. **Native semantic projection.** Add exact hover precedence, Beacon-only
   labels and control tooltips, and the five current-runtime fake-item
   projections. Lock selected/disabled/upgrade behavior and fallback semantics
   with focused native tests:
   `scripts/cargo-dev.sh test -p bbb-native beacon`.
2. **Renderer consumption proof.** Lock inventory draw-command ordering and
   deterministic offscreen pixels for the four button bases, their icons and
   the example payment items through the existing typed sprite and resident
   item consumers. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-renderer beacon` and
   `scripts/cargo-dev.sh test -p bbb-renderer hud_inventory`.
3. **Closeout.** Re-run all focused families, remove the ledger item, archive
   this contract, append the milestone index, run the standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`62b2a3f6ea33100cd3ed3a2ec5806f3f3b2af45d`. The final full gate is the
engineering checkpoint. A playable gate is required only if input or container
command routing changes; this presentation-only slice otherwise uses existing
interaction tests plus deterministic GPU readback.

The contract, native projection, and renderer proof stages are the clean commits
`11f603dc4`, `80bbea574`, and `a53da0363`.

## Failure localization and safe stops

Projection failures compare synchronized levels/payment, local selection and
cursor, generated button geometry, state precedence, localization identity,
and item-resource resolution before renderer code. Renderer failures compare
typed sprite residency, background-layer order, fake-item residency, font and
tooltip commands, then pixels.

The clean contract commit and the buildable stage-one commit are safe stops.
Any focused or gate failure is fixed forward before another slice; history is
not rewritten.
