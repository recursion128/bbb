# Simulation Publication Snapshot Cost

> Canonical scope：当前状态分类与发布边界见
> [`docs/wiki/architecture/client-boundaries.md`](wiki/architecture/client-boundaries.md)。
> 本页只保留 publication snapshot 的详细 evidence。

This document is the pre-start contract and measurement record for the
high-risk engineering slice that replaces the deep `WorldStore` clone in each
simulation publication. It refines the immutable-publication boundary in
[`simulation-render-thread-design.md`](simulation-render-thread-design.md); it
does not introduce another gameplay owner or publication path.

## Semantic Goal and Completion

`SimulationCore` remains the only mutable simulation owner. Each completed
step publishes one immutable generation that render and control readers share
through the same world `Arc` and generation. Building that generation must no
longer deep-copy unchanged chunk palette/light payloads or session-static
registry and item-profile tables.

The slice is complete when:

- unchanged columns share their complete `ChunkColumn` payload between
  generations and a block, biome, light, or block-entity update copies only
  the affected column;
- committed `RegistrySet` and individual `ItemProfiles` tables share across
  ordinary publications and copy only at their configuration/reload setter
  boundaries;
- retained generation A cannot observe any later block, light, chunk
  load/unload, entity transform/metadata, registry/reload, inventory, or UI
  mutation, while generation B observes it;
- renderer and control still hold the same world `Arc` for one generation and
  publication still follows its ordered effect sequence;
- runtime diagnostics report the most recent publication build duration;
- the before/after benchmark below demonstrates that mutation cost plus
  publication cost fell rather than merely moving to the next write; and
- focused tests, final full gate, and playable gate pass at delivered `HEAD`.

## Non-Goals

- No protocol, tick, input, renderer, serialization shape, or visible feature
  changes.
- No `RwLock`, shared mutable `WorldStore`, second canonical world, independent
  control snapshot, skipped necessary publication, or mixed-generation read.
- No configuration UI, `unsafe`, global allocator instrumentation, or wall-time
  threshold in CI.
- Entity ECS replacement and entity-tick behavior are outside this slice. The
  entity path remains explicitly measured; it must not be wrapped in a whole
  `EntityStore` COW that simply moves its current publication clone into the
  next tick mutation.

## Ownership and Frozen Integration Contract

The involved boundaries are:

- `bbb-world`: canonical mutable values, immutable sharing granularity,
  serialization compatibility, snapshot construction, and isolation tests;
- `bbb-native::simulation::publication`: generation construction, duration
  measurement, `ArcSwap`, and render/control publication ordering;
- `bbb-control`: low-cost publication-duration observation in the existing
  generation-matched counters; and
- renderer/control consumers: unchanged read-only `Arc<WorldStore>` API and no
  new copy or lock.

The frozen storage contract is:

1. `WorldStore::publication_snapshot` is the only world-value construction
   entry used by `SimulationPublisher` and the benchmark.
2. `chunks` stores one shared immutable owner per `ChunkColumn`. Read APIs keep
   returning `&ChunkColumn`; every column mutation obtains uniqueness at the
   existing `bbb-world::chunks` mutation boundary before writing. Load replaces
   or inserts one owner and unload removes one owner.
3. The committed registry set has one shared immutable owner and each item-
   profile table has its own shared owner. The resource-derived command sound-id
   list is another serde-omitted publication-shared owner. Their existing
   startup/configuration/reload setters replace or obtain uniqueness before
   mutation. Pending registry collection stays inside the registry owner and a
   committed candidate remains atomic with its item defaults; sound ids commit
   with the matching native audio resource reload acknowledgement.
4. Entity state stays in the one live hecs owner and is snapshot explicitly at
   publication. It is not placed behind whole-store COW. The baseline shows its
   publication-only cost separately from the much larger entity client-tick
   cost; any residual O(entity count) publication bookkeeping remains visible
   in the after data instead of being hidden in mutation.
5. The publication world continues to contain all other mutable client state
   (inventory/UI/counters included) by value. Those smaller owners are not
   broadened into this optimization without measurement.
6. Serde must preserve the existing `WorldStore` JSON field names and nesting;
   structural owners are an in-memory detail only.

Map color pixels later adopt the same rule at their canonical semantic owner:
the byte payload is an Arc-COW allocation with a monotonic content revision and
an exact last patch. A retained publication shares unchanged pixels, and the
first later patch copies only that map payload. Renderer fixed-page texture
residency is a derived consumer of this revision; it is never stored back into
the publication and does not alter the serialized map byte-array shape.

The live world holds the current owners while every retained generation holds
the owners it observed. A first mutation while an older generation is retained
can temporarily hold both old and new payloads for the changed column or
session-static table. Unchanged columns/tables remain single-copy. This bounds
peak memory by retained generations plus changed-owner payloads rather than by
the entire resident world per publication.

## Authoritative Evidence

There is no wire change in this slice. Local vanilla 26.1 sources establish
the behavior that storage sharing must preserve:

- `ClientPacketListener.handleLevelChunkWithLight`, `handleBlockUpdate`,
  `handleLightUpdatePacket`, `handleSetEntityData`, and `handleMoveEntity` all
  marshal mutation onto the one client packet processor thread.
- `ClientChunkCache.replaceWithPacketData`, `replaceBiomes`, `drop`, and
  `updateViewRadius` replace or mutate individual resident columns rather than
  rebuilding unrelated columns.
- `ClientLevel` owns one `ClientChunkCache`, one
  `TransientEntitySectionManager`, and one entity tick list; `tickEntities`
  mutates that live entity owner.
- `ClientPacketListener.registryAccess` is a final `RegistryAccess.Frozen`
  received with the connection cookie, matching a session/reload-scoped owner
  rather than a per-frame copy.

Repo architecture supplies the additional invariants: the effect-before-
publication gate, exact generation pairing, blocked-render isolation,
shutdown/join behavior, and single simulation-owner source guards remain those
defined in [`simulation-render-thread-design.md`](simulation-render-thread-design.md)
and [`native-client-architecture.md`](native-client-architecture.md).

## Baseline and Cost Attribution

Command:

```text
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo bench -p bbb-world --bench publication_snapshot
```

Machine: AMD Ryzen 9 7945HX (16 cores/32 threads), Linux 6.8.0-124-generic,
`rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)`. Baseline commit:
`cceab362258866df1db647d36635a9ae30652ab5`. Criterion uses ten samples, a
500 ms warm-up, and a two-second measurement window; these values are records,
not CI assertions.

| Scenario | Baseline estimate |
| --- | ---: |
| unchanged empty world | 2.687 us |
| unchanged 81 representative columns | 1.602 ms |
| unchanged 289 representative columns | 9.390 ms |
| unchanged 625 representative columns | 21.530 ms |
| unchanged 2,000 entities | 0.880 ms |
| unchanged 8,192 registry entries + 2,000 item profiles | 0.238 ms |
| one block mutation + publish, 625 columns, old snapshot held | 23.142 ms |
| entity client tick + publish, 2,000 entities, old snapshot held | 82.470 ms |
| unchanged next generation, 625 columns, old snapshot held | 23.148 ms |

The near-linear column payload growth and the equality of retained/ordinary
625-column publication localize the main cost to snapshot construction, not
`ArcSwap`. The 625-column fixture contains 24 packed block palettes and 26 sky
plus block light arrays per column. The entity-only publication is below one
millisecond in this fixture, while the full entity tick plus publication is
over 82 ms; therefore whole-`EntityStore` COW would obscure or relocate cost
without addressing the measured dominant entity tick. Session-static tables
are smaller but still pay on every generation despite changing only at
configuration/reload boundaries.

The same command on the same machine with the gated structural-sharing
milestone at `a238c083440e5df3859ed5113136d684ecad088e` produced:

| Scenario | Baseline | Structurally shared | Improvement |
| --- | ---: | ---: | ---: |
| unchanged empty world | 2.687 us | 2.522 us | 1.07x |
| unchanged 81 representative columns | 1.602 ms | 3.559 us | 450x |
| unchanged 289 representative columns | 9.390 ms | 6.493 us | 1,446x |
| unchanged 625 representative columns | 21.530 ms | 10.650 us | 2,022x |
| unchanged 2,000 entities | 0.880 ms | 0.905 ms | 0.97x |
| unchanged 8,192 registry entries + 2,000 item profiles | 0.238 ms | 2.526 us | 94.2x |
| one block mutation + publish, 625 columns, old snapshot held | 23.142 ms | 20.530 us | 1,127x |
| entity client tick + publish, 2,000 entities, old snapshot held | 82.470 ms | 83.904 ms | 0.98x |
| unchanged next generation, 625 columns, old snapshot held | 23.148 ms | 11.288 us | 2,051x |

The block result includes the first write while the previous generation is
retained, so its 1,127x reduction measures affected-column COW plus publication
rather than hiding a deferred copy. The entity-only and entity-tick reruns vary
by a few percent and retain their original cost owner: publication still
rebuilds the hecs store and the measured client tick still dominates that path.
No wall-clock value is a CI threshold.

## Implemented Ownership and Observation

`PublicationShared<T>` is a crate-private, serde-transparent `Arc<T>` owner.
Its only mutable projection is `DerefMut`, which calls `Arc::make_mut`; existing
world mutation APIs therefore cannot write into a retained generation without
first obtaining a unique owner. `WorldStore::chunks` holds one such owner per
column, `RegistrySet` has one session/reload-scoped owner, and every heap-backed
item-profile table has its own owner. The item-profile granularity preserves
the pre-existing flattened `WorldStore` serialization shape.

`SimulationPublisher` is the only native caller of
`WorldStore::publication_snapshot`. It measures construction from immediately
before the world snapshot through allocation of the complete immutable
`SimulationPublication`. The generation-matched `NetCounters` projection
reports `simulation_publication_build_nanos` and the process-lifetime peak in
`simulation_publication_build_peak_nanos`; render and control still reuse the
same world `Arc` and generation.

The performance follow-up distinguishes publication of a new world value from
publication of input/diagnostic facts. `publish_reusing_world` clones the exact
previous world `Arc` instead of invoking `WorldStore::publication_snapshot`;
the paired render/control generation uses that same pointer and the entity
snapshot observation is zero. The conservative caller permits this only for a
narrow exhaustive set of render-runtime commands. A due tick, network event,
gameplay input/control command, terrain readiness transition, or reload still
rebuilds the immutable world value. This is not whole-store COW: later mutation
continues against the single independent mutable simulation owner and pays no
deferred clone of the retained snapshot.

The measured entity follow-up retained that outer boundary and added an inner
entity-shell timer. `simulation_entity_snapshot_build_nanos` and its peak cover
only `EntityStore::clone`, while
`simulation_entity_snapshot_rebuilt_entities` reports the successfully rebuilt
hecs shells. The same generation also carries the latest/peak non-empty entity
tick batch time and its logical scanned/mutated row counts; see
[`entity-tick-publication-cost-design.md`](entity-tick-publication-cost-design.md).

Residual costs and memory behavior are explicit:

- `EntityStore` publication remains O(entity count), about 1.0 ms for the
  2,000-entity fixture, because each generation owns an isolated hecs shell and
  protocol-id handle map. The follow-up shares heavy component fields at their
  semantic owner: the 2,000-entity payload-heavy unchanged case fell from
  22.955 ms to 0.966 ms, and sparse retained metadata mutation plus publication
  fell from 22.987 ms to 1.010 ms. Transform/animation remain direct values;
  no whole-store copy moved to the next tick.
- The first registry mutation after a retained snapshot clones the complete
  `RegistrySet`; reload is rare, but its transient old-plus-new peak remains.
  Independent item-profile tables disconnect only when their setter changes
  that table.
- Section-dirty terrain rendering adds one non-serialized
  `PublicationShared<TerrainChunkRenderRevisions>` value per touched chunk.
  The outer ordered map still clones its keys and Arc handles at a world
  publication, while a block/light mutation disconnects only affected small
  revision records. On the same 625-column fixture this moves unchanged
  publication from 10.650 to 13.565 us and single-block mutation plus
  publication from 20.530 to 24.063 us. Whole-map COW is deliberately rejected:
  it would move an O(chunk count) copy to the first mutation while an old
  generation is retained.
- A retained reader keeps the old payload for each changed column/table until
  it releases that generation. Consecutive changed generations can therefore
  retain several versions of those owners, while unchanged owners remain one
  allocation. No allocator-wide byte counter was added because reliable byte
  accounting would require global allocation instrumentation outside the
  no-`unsafe`, low-overhead contract.

## Serial Phases, Validation, and Safe Stops

1. **Measurement and contract.** Add the repo-native Criterion target, capture
   the unoptimized data above, and freeze this contract. Focused validation:
   benchmark compilation/run, `bbb-world` tests, format/diff/doc checks. This
   changes a Cargo target, so commit it normally and run a full gate. Its gated
   tip is a safe stop.
2. **World structural sharing.** Introduce column and session-static sharing,
   audit every write for uniqueness, preserve serde shape, and add all old-
   generation isolation cases. Run `bbb-world` focused tests and the benchmark;
   commit only a buildable/reviewed state and run a full gate because this is
   the core architecture milestone. Its gated tip is a safe stop.
3. **Publication integration and observation.** Route
   `SimulationPublisher` through the snapshot boundary, add duration counters,
   retain same-Arc/generation and sequence/order guards, update architecture
   docs with after data and residual hotspots, and review all Arc/COW points.
   Run affected `bbb-world`, `bbb-control`, and `bbb-native` tests, then final
   full gate and `scripts/playable-gate.sh auto` at delivered `HEAD`.

Failures localize to the benchmark/contract target in phase 1, world storage
and serialization in phase 2, or native publication/counter ordering in phase
3. No phase is allowed to leave both deep and structural publication paths
selectable, and fixes use normal follow-up commits without history rewriting.
