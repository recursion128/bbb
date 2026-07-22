# Block-event packet coverage closure

## Goal and completion contract

Close the three remaining coverage cells for
`ClientboundBlockEventPacket`: decode, canonical apply, and runtime side
effect. The packet's block holder must be a valid Vanilla 26.1 block-registry
identity, while dispatch itself must follow the current block state at the
packet position. Every client `triggerEvent` family must either perform its
Vanilla behavior or have source-backed evidence that the event is a no-op.

Completion means the fixed block registry has one generated protocol identity
catalog, the current-state dispatcher covers the complete override inventory,
note blocks emit exact audio and particles (including custom skull sounds),
pistons own their client move state and presentation through completion, all
five packet-coverage dimensions are `complete`, the finished unsupported
feature is removed, and the full plus playable gates pass at the milestone
head.

## Scope and non-goals

- `bbb-protocol` owns the exact `BlockPos`, unsigned-byte, unsigned-byte, and
  block-holder wire order. A generated fixed-registry catalog rejects negative
  and out-of-range holder ids before the packet reaches world application.
- `bbb-pack` owns generated Vanilla block declaration behavior needed by piston
  pushability: destroyability, push reaction, block-entity presence, and exact
  registry order. Hand-maintained membership lists are not accepted.
- `bbb-world` owns current-state event dispatch, note-block event projection,
  piston structure resolution, canonical moving-piston records and block-state
  mutations, client ticks, moving collision, and query state for presentation.
- `bbb-native` owns audio/particle routing and moving-block scene submission. It
  reuses the existing `MovingBlockRenderState` quad path; block registry ids and
  piston rules do not move into the renderer.

This slice does not simulate server redstone power, originate piston block
events, schedule server neighbour updates, award statistics, drop items, or
emit server game events. Server packets remain authoritative for later block
updates. The only new typed skull NBT value is `note_block_sound`; unrelated
block-entity NBT closure remains outside this slice. Missing canonical
moving-piston block entities keep Vanilla's empty collision fallback rather
than receiving an inferred solid shape.

## Vanilla and integration contract

`ClientboundBlockEventPacket.STREAM_CODEC` writes `BlockPos`, `b0`, `b1`, then
`ByteBufCodecs.registry(Registries.BLOCK)`. `ClientPacketListener.handleBlockEvent`
passes all four values to `Level.blockEvent`, but Vanilla 26.1's
`Level.blockEvent(BlockPos, Block, int, int)` ignores the `Block` parameter and
calls `getBlockState(pos).triggerEvent(...)`. The packet holder therefore must
resolve successfully, but it must **not** be compared with the current block
before dispatch. The generated protocol catalog, the pack declaration catalog,
and the world block-state registry must prove the same contiguous 1,168-entry
order, including the 24 registrations expanded by the three
`WeatheringCopperBlocks.create` declarations.

The complete overriding family inventory is `BaseEntityBlock`, `ComparatorBlock`,
`NoteBlock`, and `PistonBaseBlock`. `BaseEntityBlock` delegates to the block
entity; the client-observable overrides are bell, chest/ender chest, decorated
pot, end gateway, shulker box, and spawner. Comparator reaches the base
block-entity no-op. Existing event owners stay authoritative, but one dispatcher
must select them by the current block and report whether an event performed
behavior. Valid packets may still be recorded when the selected `triggerEvent`
returns false; invalid holders never reach that record.

`NoteBlock.triggerEvent` ignores `b0` and `b1`. Tunable instruments add one
`NOTE` particle at `(x + 0.5, y + 1.2, z + 0.5)` with option value `note / 24`
and pitch `2^((note - 12) / 12)`; fixed instruments use pitch `1`. The sound is
played at the block center in `RECORDS`, volume `3`, with `nextLong()` as its
seed. A custom-head instrument reads `note_block_sound` from the skull block
entity above and returns false without audio when the value is absent.

`PistonBaseBlock.triggerEvent` accepts extend `0`, contract `1`, and drop `2`.
The structure resolver has a 12-block cap, exact slime/honey branching and
non-sticking pair, vertical/world-border bounds, the four explicit immovable
blocks, destroy/push-only/block reactions, extended-piston rejection, and
block-entity rejection. Client application creates moving-piston records for
moved blocks and the source/head, applies destroy event `2001`, and plays the
extend/contract block sound with the shared deterministic random stream.
Moving records retain moved state, direction, extending/source flags,
`progressO`, `progress`, and `deathTicks`; progress advances by `0.5` per
running client tick, then the client retains the completed entity for five
ticks before installing its final state. Moving collision and sticky-entity
motion use the canonical record; presentation interpolates the same progress
and follows `PistonHeadRenderer`'s head/source/base selection and offsets.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundBlockEventPacket.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java`,
`net/minecraft/world/level/Level.java`, the four block overrides and seven
block-entity overrides, `PistonStructureResolver.java`,
`PistonMovingBlockEntity.java`, and
`net/minecraft/client/renderer/blockentity/PistonHeadRenderer.java` under the
Vanilla 26.1 source tree.

## Serial stages and verification

1. Extend the Tree-sitter block declaration generator with one generated
   protocol registry catalog, make pack/world consumers assert the same order,
   and reject invalid packet holders. Focused tests: block catalog generator
   unit tests, generator `--check`, protocol block-event decoder tests, and
   registry identity tests. This high-risk generator/protocol milestone gets a
   full gate.
2. Replace the fan-out event application with current-state dispatch; preserve
   the already covered block-entity event families and add note-block audio,
   particle, random-order, and custom-skull behavior. Focused tests:
   `bbb-world` block-event/note tests and native effect-order tests.
3. Generate the piston behavior profiles and implement the exact bounded
   structure resolver independently of chunk mutation. Focused tests:
   generator extraction tests plus world resolver cases for limits, reactions,
   block entities, bounds, slime/honey branching, and sticky retraction. This
   generator/state-machine milestone gets a full gate.
4. Apply extension/retraction atomically, add canonical moving-piston ticks and
   collision, route destroy particles and deterministic piston audio, and
   submit interpolated source/head/moved-block meshes through the existing
   moving-block path. Focused tests: world mutation/tick/collision tests and
   native scene/audio/particle tests. The cumulative fourth commit gets a full
   gate even if this stage is split into more than one buildable commit.
5. Mark the packet row complete, correct the stale unsupported wording about
   block equality, remove the finished todo, append the milestone index, review
   the integrated diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `652dbf492` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Catalog/order failures stop before dispatch changes;
current-state or random-order failures stop before piston work; resolver
failures stop before chunk mutation; mutation/tick/presentation failures stop
before coverage closeout. Failures are fixed forward without rewriting
history.
