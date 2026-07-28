# SetEntityData packet coverage closure

## Goal and completion contract

Close the four remaining coverage cells for
`ClientboundSetEntityDataPacket`: decode, canonical apply, runtime side effect,
and tests. Every vanilla 26.1 entity-data serializer must have exactly one
explicit decode route, every received row must validate against the generated
entity inheritance schema before mutation, registry holders must bind to the
value-local frozen session identity, and entity callbacks/presentation must
remain covered by the fixed 157-by-10 entity matrix.

Completion means all five packet dimensions are `complete`, the generated
43-entry serializer catalog is mechanically tied to the decoder, the entity
matrix has no partial/deferred cells, the obsolete SetEntityData unsupported
todo is removed, generated packet coverage agrees with its override source, and
the full plus playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the terminated metadata list, serializer wire values,
  bounded nested values, and unresolved holder references.
- `bbb-world` owns generated accessor lookup, serializer/schema validation,
  registry identity binding, atomic canonical mutation, and metadata-driven
  animation/state callbacks.
- `bbb-native` reads canonical metadata through entity extraction, audio,
  particle, save/debug, and input consumers; it does not decode a parallel
  metadata stream.
- The generated entity coverage ledger remains the proof inventory for all 157
  entity types and ten behavior categories.

This slice does not implement custom serializer ids, server-private entity
state, deferred client projectile simulation, unrelated mob-specific models,
or creative ItemStack encoding. It does not weaken exact schema rejection or
replace value-local registry identity with a side table.

## Vanilla and integration contract

- `ClientboundSetEntityDataPacket.STREAM_CODEC` carries an entity id followed by
  `SynchedEntityData.DataValue` rows terminated by `0xff`.
- `DataValue.read` resolves the VarInt serializer id from
  `EntityDataSerializers` and decodes exactly that serializer's stream codec;
  the 26.1 static registration block defines 43 contiguous ids.
- `ClientPacketListener.handleSetEntityData` ignores a missing entity and calls
  `SynchedEntityData.assignValues` for a present entity. Vanilla assigns each
  value and calls the accessor callback in packet order, then calls the batch
  callback once.
- bbb rejects an unknown entity, accessor, serializer mismatch, invalid rich
  component, or unresolved registry holder before mutating metadata. Valid rows
  replace or append by accessor id, retain value-local registry
  resource/source/revision identity, then run animation-target, animation-event,
  fishing-hook, position, and local-player item-use projections from the same
  accepted packet.
- `docs/entity-coverage-26.1.json` contains 1570 cells: 1567 complete and three
  model-presentation cells not-applicable for Vanilla `NoopRenderer` types. It
  has no partial/deferred cell, and its generator tests require implementation
  plus semantic-test evidence for every complete cell.

Authoritative sources are
`net/minecraft/network/protocol/game/ClientboundSetEntityDataPacket.java`,
`net/minecraft/network/syncher/SynchedEntityData.java`,
`net/minecraft/network/syncher/EntityDataSerializers.java`, and relevant entity
`onSyncedDataUpdated` overrides under the local vanilla 26.1 source tree.

## Serial stages and verification

1. Add a reverse test deriving all serializer constant identifiers from the
   generated 43-entry catalog and requiring exactly one decoder route for each,
   while retaining the explicit unknown-serializer rejection. Focused tests:
   the named `bbb-protocol` serializer tests and non-test protocol check.
2. Mark the four packet-coverage cells complete, require the serializer and
   entity-matrix closeout evidence in the packet reverse guard, regenerate the
   packet ledger, remove the finished unsupported todo, and append the milestone
   index. Focused tests: packet/entity coverage generator tests/checks,
   `bbb-protocol` packet coverage tests, docs lint, and non-test protocol check.
3. Review the integrated diff and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `fea266182` is the pre-start checkpoint. Each stage is a clean,
buildable commit. A serializer-route mismatch stops in stage 1; a matrix or
generated-ledger mismatch stops in stage 2. Failures are fixed forward without
rewriting history.
