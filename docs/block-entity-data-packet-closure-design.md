# Block-entity-data packet coverage closure

## Goal and completion contract

Close the remaining decode, canonical-apply, and runtime-side-effect coverage
cells for `ClientboundBlockEntityDataPacket`. The packet must carry a valid
Vanilla 26.1 `BlockEntityType` holder and one complete non-null CompoundTag.
World application must update only an already loaded block entity at the
packet position whose type is identical to the packet type; an absent or
different type is ignored without decoding or retaining the payload.

Completion means protocol decode rejects invalid registry ids, null or
non-compound roots, malformed bounded NBT, and trailing bytes; a matching
canonical record is replaced atomically while absent and mismatched records
remain unchanged; every retained typed projection is selected only by its
own block-entity type; the existing editor and renderer consumers observe the
replacement through the canonical record; all five packet-coverage dimensions
are `complete`; the milestone is indexed; and the full plus playable gates
pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the exact `BlockPos`, fixed block-entity registry holder,
  and terminal trusted CompoundTag wire order. It preserves the validated raw
  NBT bytes for the semantic world decoder.
- The existing generated `BlockEntityType` catalog remains the only fixed
  holder-id authority. The packet decoder and semantic fixtures consume its
  constants and catalog length rather than a second table or numeric ids.
- `bbb-world` owns loaded-record identity, atomic replacement, generic raw/NBT
  retention, type-selected sign/hanging-sign, vault, decorated-pot, banner,
  end-gateway, spawner, command-block, structure-block, jigsaw, test-block,
  test-instance-block, and skull-note-sound projections, counters, diagnostics,
  and authoritative editor refreshes.
- `bbb-native` continues to consume the canonical projections through the
  existing sign/banner/decorated-pot/spawner/end-gateway scenes, vault event
  context, skull note-block audio, and editor state. This slice adds a complete
  network-event-to-presentation regression rather than another block-entity
  cache.
- The nullable CompoundTag parser shared by level-chunk block-entity entries
  may receive the same root/count/depth hardening so the two Vanilla NBT codecs
  cannot disagree. No other `LevelChunkWithLight` lifecycle changes belong to
  this slice.

This slice does not implement new block-entity fields, inventory contents,
client tick effects, renderers, render-pipeline refinements, or server-side
block-entity behavior. In particular it does not reopen the deferred
trial-spawner `SpawnData` presentation, player-head profile skin, sign glyph
effects, banner depth-write mode, chest calendar texture, block-entity
culling, or idle/tick effects. It does not infer or create a record from the
block state when the packet arrives; `BlockUpdate` and level-chunk ingestion
remain the owners of record creation and removal.

## Vanilla, wire, and integration contract

`ClientboundBlockEntityDataPacket.STREAM_CODEC` writes
`BlockPos.STREAM_CODEC`, then
`ByteBufCodecs.registry(Registries.BLOCK_ENTITY_TYPE)`, then
`ByteBufCodecs.TRUSTED_COMPOUND_TAG`. The registry codec rejects a holder id
absent from the active built-in registry. `TRUSTED_COMPOUND_TAG` composes the
non-null tag codec with a CompoundTag type check: the EndTag/null sentinel and
every non-compound root are invalid. The CompoundTag is terminal, so one
complete root must consume the packet. bbb keeps its existing defensive
depth, collection-count, array-count, and framed-packet byte limits even
though Vanilla's trusted codec uses an unlimited heap accounter.

`ClientPacketListener.handleBlockEntityData` calls
`level.getBlockEntity(pos, packet.getType())`. `BlockGetter.getBlockEntity`
returns a value only when an entity already exists at the position and its
`BlockEntityType` object is identical to the requested type. Missing chunks,
missing entities, and type mismatches therefore ignore the update; the packet
never creates an entity and never changes its type. On a match, Vanilla calls
`loadWithComponents` on that concrete block-entity class. Accordingly bbb
first resolves the matching canonical record, then decodes a complete
replacement off to the side, and mutates only after every retained projection
has succeeded. A malformed matching payload reports an apply error and
preserves the old record.

Concrete Vanilla block-entity classes read only their own fields. bbb's
generic NBT summary is retained for all matching types, but each specialised
projection is decoded only for its exact generated type: sign text accepts
`SIGN` and `HANGING_SIGN`; every other projection accepts its one catalog type.
This prevents unrelated NBT keys from populating or invalidating another
block-entity family. The four open editor snapshots refresh only after a
matching successful replacement, mirroring the authoritative command-block
GUI refresh and the existing bbb editor acknowledgement contract. Existing
world-to-native scene extraction reads the same replaced record on the next
runtime publication; it does not receive a parallel packet notification.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundBlockEntityDataPacket.java`,
`net/minecraft/network/codec/ByteBufCodecs.java`,
`net/minecraft/network/FriendlyByteBuf.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java`, and
`net/minecraft/world/level/BlockGetter.java` under the Vanilla 26.1 source
tree, plus the relevant concrete `BlockEntity.loadAdditional` implementations
for the retained typed projections.

## Serial stages and verification

1. Harden the packet decoder around the generated registry and required
   terminal CompoundTag, add exact raw preservation, and bound depth,
   compound/list/array counts before allocation or iteration. Reuse the same
   validator for nullable level-chunk block-entity tags without changing their
   nullable contract. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-protocol packets::chunks::tests` and
   `scripts/cargo-dev.sh check -p bbb-protocol`. This protocol milestone gets
   a full gate.
2. Centralize construction of a typed `BlockEntityRecord`, select every typed
   projection by generated type identity, and make update application resolve
   an existing matching record before decode and replace it atomically.
   Focused tests cover all generated type ids, absent/mismatched identity,
   malformed replacement preservation, type-projection isolation, counters,
   and editor refreshes with
   `scripts/cargo-dev.sh test -p bbb-world`.
3. Repair semantic fixtures that previously depended on synthetic creation or
   cross-type replacement, and add an ordered
   `LevelChunkWithLight`/`BlockUpdate`/`BlockEntityData` event regression whose
   next native extraction contains the updated sign presentation. Run focused
   `bbb-net` event/probe tests and
   `scripts/cargo-dev.sh test -p bbb-native block_entity_data` plus the
   affected block-entity scene tests.
4. Mark the packet row complete, regenerate and check the packet ledger,
   remove only a TODO actually closed by this slice, append the milestone
   index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh live` and `scripts/playable-gate.sh auto`.

The fully gated `99de289a5` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Registry/NBT failures stop before world mutation; identity or
atomicity failures stop before runtime evidence; native extraction failures
stop before coverage closeout. A failed full gate is localized from the latest
fully gated checkpoint only when focused tests do not identify it. Failures
are fixed forward without rewriting history.
