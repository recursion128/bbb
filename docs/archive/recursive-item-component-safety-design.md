# Recursive ItemStack component safety foundation

## Goal and completion contract

Make the four Vanilla 26.1 components that recursively contain
`ItemStackTemplate` values safe to decode and consume from untrusted packet
input:

- `USE_REMAINDER`;
- `CHARGED_PROJECTILES`;
- `BUNDLE_CONTENTS`; and
- `CONTAINER`.

Completion means the protocol decoder applies one shared recursion budget to
every nested `ItemStackTemplate`, accepts the same non-zero network counts as
Vanilla, and rejects an over-budget value before descending further. The
existing recursive bundle model, charged-projectile tooltip, and debug
persistent-projection paths must either use the same semantic budget or an
equivalent non-recursive traversal. Focused tests must cover the exact boundary,
budget restoration after an error, and actual stack consumption at the accepted
boundary on a production-sized packet worker thread. Recursive consumer tests
run on explicitly sized threads with their own byte budgets.

The client budget is 32 nested templates. Vanilla does not impose this limit;
it is a bbb defensive limit for untrusted recursive data. A measured debug-build
full packet decode at the limit must remain below 1.5 MiB on a 2 MiB thread so
the semantic constant is backed by a byte-denominated guard and retains caller
headroom on the Tokio worker threads where packet decoding runs.

## Non-goals

- This slice does not implement the missing generic data-component network
  encoder.
- It does not claim lossless canonical or persistent semantics for arbitrary
  component patches nested in these four values.
- It does not close any component-dependent packet coverage cell or mark any of
  the four component rows complete.
- It does not apply the persistent `ItemStackTemplate` count range of `1..=99`
  to the network codec.
- It does not address independent recursive formats such as SNBT trees or
  holder sets.

## Ownership and integration boundary

- `bbb-protocol::codec::Decoder` owns reusable, scoped recursion accounting.
  Entering a scope increments the counter, over-budget entry fails before its
  callback runs, and every success or error restores the previous counter.
- `bbb-protocol::packets::data_components` owns the item-template limit and
  applies it exactly once per nested template. The root packet stack is not a
  template level.
- `bbb-item-model` owns bounded bundle-weight and charged-projectile tooltip
  traversal. Programmatically constructed summaries receive the same safe
  behavior as packet-origin summaries.
- `bbb-native` owns bounded debug persistent projection of nested templates.
  Budget exhaustion returns the existing unsupported-projection result rather
  than emitting a partial value.
- The item-component evidence overrides may record the new safety evidence as
  `partial`; they may not upgrade a semantic dimension to `complete`.

## Vanilla 26.1 evidence

- `DataComponents.java` registers `USE_REMAINDER` with
  `UseRemainder.STREAM_CODEC`, `CHARGED_PROJECTILES` and `BUNDLE_CONTENTS` with
  unbounded-list wrappers around `ItemStackTemplate.STREAM_CODEC`, and
  `CONTAINER` with `ItemContainerContents.STREAM_CODEC`.
- `ItemContainerContents.STREAM_CODEC` uses a list maximum of 256.
- `ItemStackTemplate.STREAM_CODEC` writes a non-empty item holder, VarInt count,
  and `DataComponentPatch.STREAM_CODEC`.
- The `ItemStackTemplate` network constructor rejects zero count and air, but
  accepts negative counts. Its persistent MapCodec separately constrains count
  to `1..=99`.
- `DataComponentPatch.STREAM_CODEC` carries positive component entries as type
  plus value and negative entries as type only, so any of the four values can
  recursively contain another one.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/ItemStackTemplate.java`,
`world/item/component/UseRemainder.java`,
`world/item/component/ChargedProjectiles.java`,
`world/item/component/BundleContents.java`, and
`world/item/component/ItemContainerContents.java`.

## Serial stages and validation

1. Add scoped decoder recursion accounting, align network count validation, and
  prove depth 32 succeeds while 33 fails deterministically. Measure real stack
  use through the packet dispatcher at depth 32 and keep it below the byte
  budget on a production-sized worker thread.
2. Bound recursive bundle-model, tooltip, and debug-projection traversal while
   preserving existing results below the limit. Add focused boundary and
   programmatic-summary tests.
3. Record only partial component-ledger evidence, narrow the remaining safety
   TODO without deleting unrelated recursive work, review the integrated diff,
   and run the per-commit checks.

The slice is a high-risk untrusted-input milestone. Its integration commit must
pass the full repository gate before another implementation slice starts.
