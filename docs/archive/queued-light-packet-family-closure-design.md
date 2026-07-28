# Queued light packet family closure

## Goal and completion contract

Close the shared client-light lifecycle for
`ClientboundLevelChunkWithLightPacket`, `ClientboundLightUpdatePacket`, and
`ClientboundForgetLevelChunkPacket`. Chunk sections and block entities publish
immediately, while initial and incremental light data enter one world-owned
FIFO and are applied after the current network drain with Vanilla's bounded
per-frame policy. Unloading a chunk removes every still-pending light value for
that chunk before it can reach terrain presentation.

Completion means the three packet rows have all applicable dimensions marked
`complete`; chunk/light decode rejects impossible catalog ids, counts, masks,
and data-layer cardinalities before allocation or canonical mutation; the
active dimension fixes the section and light-section domains; queued updates
preserve packet order; and terrain revisions cause the renderer worker/cache to
observe only applied light. The two queued-light todos are then removed, and
the full plus playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the two-megabyte section buffer, generated heightmap and
  block-entity type ids, bounded maps/lists/bitsets, 2,048-byte data layers,
  and exact packet termination.
- `bbb-world` owns dimension-aware section validation, the runtime-only light
  FIFO, canonical light merge, terrain section/neighbour revisions, unload
  cleanup, diagnostics, and queue query APIs.
- `bbb-native` polls the queue after the single network-event drain and before
  world publication/render extraction. `bbb-net` probe uses the same world
  poll API rather than a parallel light switch.
- Existing terrain mesh workers and GPU residency continue to consume the
  canonical terrain revision graph; no second renderer light cache is added.

This slice does not implement a voxel light-propagation engine, change biome
visual fields, add section-granular visibility, alter block model baking, or
complete unrelated block-entity renderers. It does not serialize pending light
work into canonical snapshots or move canonical mutation to the render thread.

## Vanilla and integration contract

- `ClientboundLevelChunkWithLightPacket` reads fixed `int` x/z, a
  `ClientboundLevelChunkPacketData`, then `ClientboundLightUpdatePacketData`.
  `ClientboundLevelChunkPacketData` uses the `Heightmap.Types` enum map, a
  section byte buffer capped at 2 MiB, registry-backed block-entity type ids,
  and nullable NBT update tags.
- `ClientboundLightUpdatePacketData` carries four long-array bitsets followed
  by sky and block data-layer lists; every non-empty layer is exactly 2,048
  bytes. The active `LevelLightEngine` consumes only its dimension-derived
  light-section count.
- `ClientPacketListener.handleLevelChunkWithLight` replaces chunk data first,
  then calls `ClientLevel.queueLightUpdate`; its queued closure applies light,
  enables chunk light, and marks the full chunk neighbourhood ready.
  `handleLightUpdatePacket` enqueues the same apply path with section-neighbour
  rebuild enabled.
- `ClientLevel.pollLightUpdates` snapshots queue length. For fewer than 1,000
  entries it consumes `max(10, size / 10)`; at 1,000 or more it consumes the
  complete snapshot. Newly enqueued work waits for a later poll.
- `handleForgetLevelChunk` drops canonical chunk data and queues light-engine
  removal for every light section. Because bbb has no separate propagating
  light engine, its equivalent is to remove both canonical light and pending
  FIFO values for the unloaded chunk, counting discarded incremental packets
  as ignored.

The authoritative sources are
`net/minecraft/network/protocol/game/ClientboundLevelChunkWithLightPacket.java`,
`ClientboundLevelChunkPacketData.java`,
`ClientboundLightUpdatePacketData.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java` methods
`handleLevelChunkWithLight`, `handleLightUpdatePacket`, `applyLightData`,
`readSectionList`, and `handleForgetLevelChunk`, plus
`net/minecraft/client/multiplayer/ClientLevel.java` methods
`queueLightUpdate` and `pollLightUpdates`.

## Serial stages and verification

1. Tighten protocol decode with generated catalog/domain limits, exact
   data-layer cardinality, and malicious-boundary tests. Focused checks are the
   named `bbb-protocol` chunk tests and non-test protocol check.
2. Add the world-owned FIFO, dimension-aware atomic validation, Vanilla poll
   budget, unload/session cleanup, queue queries, and world tests. Wire
   `apply_play_packet`, native simulation, and probe through the same API; add
   focused world/net/native ordering tests and affected non-test checks.
3. Mark the three packet rows closed with reverse evidence, regenerate the
   packet ledger, remove the two finished todos, update architecture/history,
   and run packet-ledger tests.
4. Review the integrated diff, run `scripts/cargo-dev.sh gate`, then
   `scripts/playable-gate.sh auto`.

The fully gated `7c4fdab1c` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol validation failure stops before queue work; queue
ordering or cleanup failure stops before ledger closeout. Failures are fixed
forward without rewriting history.
