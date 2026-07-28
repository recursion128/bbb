# Block-update packet family coverage closure

## Goal and completion contract

Close the remaining coverage cells for `ClientboundBlockUpdatePacket` and
`ClientboundSectionBlocksUpdatePacket`. Both packets must reject block-state
ids outside the fixed Vanilla 26.1 state registry before world application,
then apply each accepted state through one server-verified mutation path. That
path must preserve local prediction ordering, maintain counted chunk data and
heightmaps, reproduce Vanilla block-entity removal/creation rules, and advance
the exact terrain-section revisions consumed by the native remesh scheduler.

Completion means the fixed block-state range and each block's block-entity
type are generated or mechanically checked against the checked-in Vanilla
catalogs, single and section updates share the same canonical mutation,
replacement removes stale block-entity payloads while valid new entity blocks
receive an empty canonical record, copper chest/statue conversions preserve
their Vanilla entity, the native terrain cache demonstrably rebuilds only the
affected section subset, both packet rows have all five coverage dimensions
`complete`, the finished unsupported todo is removed, and the full plus
playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the exact fixed block-state identity range, strict
  VarInt/VarLong validation, absolute `BlockPos` single update, and section
  position plus ordered packed-change decode.
- The block declaration generator owns the authoritative mapping from every
  Vanilla block registry entry to its optional `BlockEntityType` registry id.
  `bbb-world` consumes that mapping; it does not maintain a second hand-written
  block-entity membership table.
- `bbb-world` owns prediction-aware server-state application, section update
  ordering, palettes/counts/heightmaps, canonical block-entity record
  lifecycle, moving-piston cancellation, and terrain content/mesh revisions.
- `bbb-native` continues to own extraction, bounded mesh work, cache
  replacement, and renderer upload. This slice adds an integration regression
  around the existing revision contract rather than a second invalidation
  channel.

This slice does not simulate server neighbour/redstone updates, client light
propagation, block placement callbacks, server block drops, block sounds or
particles, or server-authoritative collision outcomes. It does not close
`BlockEntityData`: exact type-gated NBT application and the missing specialised
block-entity presentations remain their own vertical work. It does not change
the local prediction authority boundary or add in-game configuration.

## Vanilla, wire, and integration contract

`ClientboundBlockUpdatePacket.STREAM_CODEC` writes `BlockPos.STREAM_CODEC`
followed by `ByteBufCodecs.idMapper(Block.BLOCK_STATE_REGISTRY)`. The mapper
uses `byIdOrThrow`, so a negative or out-of-range state id is not a valid
packet. `ClientboundSectionBlocksUpdatePacket` writes `SectionPos`, a VarInt
count, then ordered VarLong values `(Block.getId(state) << 12) | packedPos`.
The low twelve bits are local `x:z:y` nibbles; the remaining high bits are the
same global block-state id. Its constructor looks up every decoded id in
`Block.BLOCK_STATE_REGISTRY`; bbb rejects an absent lookup at the protocol
boundary instead of carrying Vanilla's eventual null failure into runtime.
The physically possible section change set is bounded at 4,096 entries before
allocation.

`ClientPacketListener.handleBlockUpdate` and `handleChunkBlocksUpdate` both
call `ClientLevel.setServerVerifiedBlockState(pos, state, 19)`, with section
changes applied in packet order. `BlockStatePredictionHandler` first records a
known server state for a pending prediction and defers the visible mutation
until its acknowledgement; otherwise `Level.setBlock` owns the mutation.
Therefore both bbb packet arms must call the same prediction-aware helper and
must count deferred known-state replacement as applied without prematurely
changing the visible block.

`LevelChunk.setBlockState` returns without side effects for an unchanged state.
For a change it updates all four maintained heightmaps, section emptiness/light
status, and light checks. bbb already owns counted palettes and the four client
heightmaps; light-engine simulation remains outside this slice. When the block
kind changes, Vanilla removes an old block entity unless the new block's
`shouldChangedStateKeepBlockEntity` accepts it. Vanilla 26.1 has exactly two
overrides: all copper-chest conversions and all copper-golem-statue
conversions. If the new state has a block entity, Vanilla removes any record
whose type rejects the new state and creates a default record when none
remains. The generated `BlockEntityType.java` valid-block mapping supplies the
expected type; a newly created bbb record contains only position and type until
an authoritative chunk or `BlockEntityData` payload supplies NBT.

After a real state change, `Level.setBlocksDirty` and flag `2`'s
`sendBlockUpdated` reach `LevelRenderer`; `blockChanged` marks every render
section intersecting the one-block halo around the changed position. The
existing `mark_terrain_block_changed` contract is equivalent: only horizontal
chunk neighbours crossed by local coordinate 0/15 participate, and
`(y - 1) / y / (y + 1)` select vertical sections. Native consumes these exact
per-section revisions, preserves unaffected cached section layers, discards
stale worker results by revision key, and uploads the replacement layers.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundBlockUpdatePacket.java`,
`ClientboundSectionBlocksUpdatePacket.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java`,
`ClientLevel.java`, `net/minecraft/world/level/Level.java`,
`net/minecraft/world/level/chunk/LevelChunk.java`,
`net/minecraft/world/level/block/entity/BlockEntityType.java`,
`CopperChestBlock.java`, `CopperGolemStatueBlock.java`, and
`net/minecraft/client/renderer/LevelRenderer.java` under the Vanilla 26.1
source tree.

## Serial stages and verification

1. Move the checked-in fixed block-state report under protocol registry
   ownership, expose a mechanically checked contiguous state count, extend the
   Tree-sitter block declaration extraction with exact optional block-entity
   type identity, regenerate its protocol catalog, and reject invalid ids in
   both packet decoders. Focused tests: block catalog extractor/generator tests,
   generator `--check`, block-state catalog tests, and chunk packet decode
   boundary tests. This generator/protocol milestone gets a full gate.
2. Make the world registry resolve expected block-entity types from generated
   block metadata; route single/section updates through strict prediction-aware
   state application; remove, preserve, validate, or create records in Vanilla
   order; retain packet-order and exact count/revision behavior. Focused tests:
   `bbb-world` chunk tests for invalid ids, ordinary replacement, empty
   creation, mismatched pre-existing records, same-block changes, both copper
   preservation families, prediction deferral, section ordering, and 3D halo
   revisions.
3. Add a native terrain-runtime regression that applies both packet forms and
   proves the changed section layers are rebuilt/uploaded while unaffected
   cached sections remain reusable. Focused tests: the terrain runtime subset
   plus existing native world-event lifecycle tests.
4. Mark both packet rows complete, remove the finished general block-entity
   record-removal todo, append the milestone index, review the integrated diff,
   and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `b83c676bb` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Catalog/order failures stop before decoder changes; protocol
failures stop before world mutation; lifecycle/revision failures stop before
native integration; runtime cache failures stop before coverage closeout.
Failures are fixed forward without rewriting history.
