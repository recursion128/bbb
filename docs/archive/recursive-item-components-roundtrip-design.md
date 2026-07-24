# Recursive ItemStack components round-trip closure

## Goal and completion contract

Close all six item-component coverage cells for the four Vanilla 26.1 values
whose network and persistent codecs recursively contain `ItemStackTemplate`:

- `USE_REMAINDER`;
- `CHARGED_PROJECTILES`;
- `BUNDLE_CONTENTS`; and
- `CONTAINER`.

Completion means the shared protocol descriptor owns exact encode/decode for
each wrapper, including ordinary (not length-delimited) nested component
patches, the existing depth-32 defensive budget, non-empty item holders,
network count rules, list bounds, and sparse container slots. Canonical state
must preserve every value needed to reproduce the wrapper wire value,
including a container's trailing empty-slot span. Creative untrusted encoding
must additionally enforce the persistent `ItemStackTemplate` count range.

Persistent SNBT projection must reproduce the four persistent codecs for every
nested component value supported by the component descriptor registry. If any
nested addition or removal cannot be projected persistently, the whole
template projection fails instead of silently dropping that entry. The
existing bundle model/interaction, charged-projectile model/tooltip,
container predicate/tooltip, and generic component-presence consumers remain
the observable client owners and receive the exact retained nested stacks.

## Non-goals

- This slice does not complete a component value nested inside a template when
  that component's own coverage row is still partial. Network encoding rejects
  such a tree through the shared descriptor registry; persistent projection
  rejects it through the strict nested-patch path.
- It does not increase the defensive depth-32 or list-size budgets established
  by the recursive safety foundation.
- It does not locally predict `UseRemainder.convertIntoRemainder`. Stack
  mutation after use remains server-authoritative in the current external
  multiplayer client; component presence is still observable through the
  item-model condition path.
- It does not add creative inventory UI, new bundle interaction behavior, or
  new container screens.

## Ownership and frozen integration surface

- `bbb-protocol::packets::data_components` owns
  `ItemStackTemplateSummary`, the recursive preflight, exact container slot
  span, and ordinary versus delimited patch encoding.
- `bbb-protocol::packets::data_components::catalog` owns the four descriptor
  callbacks. A callback validates only its wrapper and delegates every nested
  patch entry back to the shared descriptor registry.
- `bbb-protocol::packets::serverbound` owns creative-only persistent range
  validation. Nested template counts must be in `1..=99`; the general network
  encoder continues to accept every non-zero `int`.
- `bbb-native::input` owns persistent SNBT projection. The current best-effort
  top-level debug projection is unchanged, while nested
  `ItemStackTemplate.CODEC` projection uses a new exact helper that rejects
  unprojectable entries.
- Existing `bbb-item-model`, `bbb-world`, and `bbb-native` consumers keep their
  public APIs. `DataComponentPatchSummary` gains only a serde-defaulted exact
  container slot-span field; no renderer or net API changes.

For `CONTAINER`, `container_slots` stays the ordered sparse non-empty view and
`container_items` stays its non-empty item projection.
`container_slot_count` is the exact wire list length. Encoding requires:

- `container_item_count == container_items.len() == container_slots.len()`;
- strictly increasing, unique slot indices;
- every slot index below `container_slot_count`, which is at most 256;
- `container_items` equals the items projected from `container_slots`.

An all-empty wire container may retain a non-zero slot count; the persistent
codec canonicalizes that value back to its empty sparse form.

## Vanilla 26.1 evidence

- `DataComponents.java` registers all four values with explicit persistent and
  network codecs and `cacheEncoding()`.
- `UseRemainder.STREAM_CODEC` is one `ItemStackTemplate.STREAM_CODEC`.
- `ChargedProjectiles.STREAM_CODEC` and `BundleContents.STREAM_CODEC` are
  ordinary unbounded-list wrappers around `ItemStackTemplate.STREAM_CODEC`.
- `ItemContainerContents.STREAM_CODEC` writes a list of optional templates with
  maximum size 256. Its persistent codec writes sparse `{slot, item}` records
  and canonicalizes the slot span to the highest non-empty slot plus one.
- `ItemStackTemplate.STREAM_CODEC` writes item holder, VarInt count, and
  `DataComponentPatch.STREAM_CODEC`. Its constructor rejects count zero and
  `minecraft:air`, while `ItemStackTemplate.MAP_CODEC` restricts persistent
  counts to `1..=99`.
- `ItemStack.OPTIONAL_UNTRUSTED_STREAM_CODEC`, used by
  `ServerboundSetCreativeModeSlotPacket`, uses
  `DataComponentPatch.DELIMITED_STREAM_CODEC` only for the outer stack. Nested
  template patches remain ordinary and therefore have no per-value length.
- `ChargedProjectiles.addToTooltip`, `BundleContents`/`BundleItem`, and
  `ItemContainerContents.addToTooltip` consume cloned nested stacks; component
  patches participate in stack equality, details, weight, selected icons, and
  predicate results.

Authoritative sources are under
`~/Work/mc-code/sources/26.1/net/minecraft/`, notably
`core/component/DataComponents.java`,
`core/component/DataComponentPatch.java`, `world/item/ItemStack.java`,
`world/item/ItemStackTemplate.java`, and the four component classes named
above.

## Serial stages and validation

1. Extend the canonical container projection, factor ordinary versus delimited
   patch encoding, add the four descriptor callbacks, and validate recursive
   templates before emitting bytes. Focused protocol tests cover each exact
   wrapper, nested supported patches, depth 32/33, negative/zero counts,
   invalid/air holders, list bounds, empty values, sparse/trailing container
   slots, unsupported inner components, and outer-only length framing.
2. Apply creative persistent count validation recursively and make nested
   persistent projection exact. Focused serverbound/native tests prove
   `1..=99`, supported nested component SNBT, unsupported-entry rejection, and
   unchanged best-effort top-level debug behavior. Existing item-model and
   publication tests prove consumers and clone isolation still see the exact
   tree.
3. Upgrade the four sparse coverage overrides, remove their encoder TODO range,
   narrow the recursive component ledger text, and add the milestone archive
   entry. Run docs/item-component generation checks, docs lint, formatting,
   focused crate tests, and diff-check before the closeout commit.

The protocol stage and consumer stage are independently buildable commits and
safe stopping points. This is a high-risk codec engineering slice: the full
repository gate runs at the final tip (and immediately after any unexpected
focused-test failure) before another slice starts. A failure in wrapper bytes,
creative admission, persistent projection, or coverage generation is localized
by the corresponding stage tests; history is fixed forward.
