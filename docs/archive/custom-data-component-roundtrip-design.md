# CustomData ItemStack component round-trip

## Goal and completion contract

Close all six item-component coverage dimensions for Vanilla 26.1
`CUSTOM_DATA`.

Completion means the generated component descriptor owns the exact
persistent-fallback network encoder; inbound values accept only the unnamed
non-null `CompoundTag` root required by `CustomData.CODEC`; outbound creative
ItemStacks preserve that compound through
`DataComponentPatch.DELIMITED_STREAM_CODEC`; and the existing canonical,
persistent-SNBT, item-model predicate, and publication paths have named
semantic-test evidence. Missing, scalar, or otherwise non-compound values must
be rejected before any outbound patch bytes are appended.

## Non-goals

- This slice does not encode component types 5 through 109.
- It does not mark `SERVERBOUND_SET_CREATIVE_MODE_SLOT` complete or remove the
  remaining component-rich encoding gap.
- It does not introduce a second NBT tree, component catalog, or generic
  dynamic component value.
- It does not widen bbb's existing network-NBT depth/count limits or claim
  Java UTF-16 lone-surrogate preservation.
- It does not change Vanilla's exact/partial custom-data predicate semantics.

## Ownership and integration boundary

- `bbb-protocol::nbt_summary` remains the sole binary NBT tree encoder/decoder.
  The component callback reuses its bounded encoder and existing depth budget.
- `bbb-protocol::packets::data_components::catalog` owns both the strict
  `CustomData` compound-root validation and its descriptor encode callback.
- `bbb-protocol::packets::data_components` owns canonical patch-shape
  validation and includes `custom_data` in the expected summary for type 0.
- `bbb-native` retains persistent SNBT projection ownership and rejects any
  locally constructed non-compound `custom_data`.
- `bbb-world` and `bbb-item-model` retain the existing canonical publication
  and exact/partial component-predicate consumers. This slice cites their
  focused tests rather than adding parallel state.
- The sparse item-component coverage override is the only hand-maintained
  ledger input.

## Vanilla 26.1 evidence

- `DataComponents.java` registers `CUSTOM_DATA` with
  `persistent(CustomData.CODEC)` and no explicit `networkSynchronized` codec.
- `DataComponentType.Builder.build` therefore uses
  `ByteBufCodecs.fromCodecWithRegistries(CustomData.CODEC)`.
- `CustomData.CODEC` maps `CustomData.COMPOUND_TAG_CODEC`; the value must be a
  `CompoundTag`, while `CompoundTag.CODEC` preserves the tag tree through
  `NbtOps`.
- `ByteBufCodecs.fromCodecWithRegistries` serializes the codec result through
  the generic unnamed NBT tag stream with the default NBT accounter.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` independently wraps that value
  in `registryFriendlyLengthPrefixed(Integer.MAX_VALUE)`.
- `CustomData.matchedBy` uses `NbtUtils.compareNbt(expected, actual, true)`;
  bbb's existing exact/partial item-model predicates retain their current
  compound-key and list-order comparison behavior.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`core/component/DataComponentType.java`,
`world/item/component/CustomData.java`,
`network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Tighten the descriptor decoder to require a non-null compound root and
   attach the existing byte/depth-bounded NBT encode callback. Focused protocol
   tests cover exact bytes, round-trip, and scalar/null rejection. Safe stop:
   no creative packet/ledger claim is made until descriptor tests pass.
2. Add `custom_data` to canonical patch preflight and prove exact delimited
   framing plus atomic rejection for missing/non-compound summaries. Focused
   protocol packet tests prove the creative-slot integration through the
   existing encoder.
3. Tighten persistent SNBT projection, audit named world/item-model tests,
   close the six ledger cells, narrow the remaining creative encoding TODO,
   and add the archive index entry. Run protocol, native, world, and
   item-model focused tests plus all per-commit checks.

This is protocol/NBT high risk: commit the complete slice, then run
`scripts/cargo-dev.sh gate`. Any failure is fixed forward before the next
slice.
