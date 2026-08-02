# Entity Tick And Publication Cost

> Canonical scope：当前状态边界见
> [`docs/wiki/architecture/client-boundaries.md`](wiki/architecture/client-boundaries.md)，
> 性能方法见
> [`docs/wiki/architecture/performance-and-validation.md`](wiki/architecture/performance-and-validation.md)。
> 本页只保留已完成工程的详细证据。

This document is the pre-start contract and measurement record for the
high-risk engineering slice that reduces the end-to-end cost of one entity
client tick followed by an immutable simulation publication. It refines the
entity and publication boundaries in
[`native-client-architecture.md`](native-client-architecture.md),
[`simulation-render-thread-design.md`](simulation-render-thread-design.md), and
[`simulation-publication-snapshot-design.md`](simulation-publication-snapshot-design.md).

## Semantic Goal And Completion

`SimulationCore` remains the only mutable owner of canonical entity state. A
client tick must retain the vanilla 26.1 entity, interpolation, animation, and
side-effect order while avoiding unrelated component work and repeated derived
lookups. The immutable generation published after that work must remain the
single world `Arc` shared by render and control readers.

The slice is complete when:

- entity tick systems query only the component families and derived inputs they
  consume, without projecting complete `EntityState` values or rebuilding
  per-tick global maps for unrelated entities;
- sparse entity mutation does not require cloning every entity payload before
  the next generation becomes visible, unless measurement proves the remaining
  full ECS publication is no longer material and the residual boundary is
  recorded explicitly;
- retained generation A remains immutable across every entity mutation family,
  while generation B observes the mutation;
- tick effects precede the first publication that exposes their canonical
  state, and renderer/control continue to share one world `Arc` and generation;
- same-machine Criterion results show a significant improvement for the
  representative 2,000-entity tick-plus-publication workload without moving the
  cost into the next mutation, render extraction, or retained-generation drop;
- generation-matched runtime observation reports recent/peak entity client-tick
  and entity-snapshot construction time plus scanned, mutated, and rebuilt
  entity counts; and
- focused tests, the declared milestone full gates, and the final full and
  playable gates pass at the delivered `HEAD`.

## Non-Goals

- No protocol, tick cadence, input, rendering, or server-authority behavior
  changes.
- No shared mutable `WorldStore`, second canonical entity world, remote mob AI,
  pathfinding, server physics, damage, combat result, or server RNG simulation.
- No skipped required tick/publication, reduced entity behavior, removed
  diagnostics, hand-written entity-type membership table, bare type ids, or
  parallel metadata schema.
- No whole-`EntityStore` COW that merely defers a complete copy to the next
  mutation.
- No `unsafe`, global allocator instrumentation, configuration UI, Rayon, or
  additional mutable ECS owner. Concurrency changes require separate measured
  evidence and are outside this slice.

## Ownership And Integration Boundary

- `bbb-world` owns the live `EntityStore`, its narrow hecs components, client
  tick systems, immutable entity snapshot representation, dirty/revision state,
  isolation tests, and logical entity work counts.
- `bbb-native` owns wall-clock observation around the existing client-tick and
  publication sequence, publishes the observations in the same generation, and
  preserves the effect-before-publication gate, deadline scheduler, blocked-
  render isolation, shutdown/join, and panic propagation.
- `bbb-control` owns only the derived `NetCounters` fields. It receives no
  entity copy and continues to pair its snapshot with the exact render world
  `Arc` and generation.
- `bbb-protocol`, `bbb-renderer`, and `bbb-net` behavior and wire/API shapes are
  not changed unless a focused regression reveals an existing owner violation.

Any immutable entity representation is a generation derived from the one live
canonical `EntityStore`; it is never independently mutable. Entity ids may be
reused only after every live index, revision, and dirty owner for the removed
entity has been cleared or replaced.

## Vanilla 26.1 Evidence

Local sources under `~/Work/mc-code/sources/26.1/` define the behavior that the
optimized systems must preserve:

- `ClientLevel.tickEntities`, `tickNonPassenger`, and `tickPassenger` tick every
  tracked, unfrozen entity exactly once through the non-passenger/passenger
  graph, save old transform state before `tickCount++`, and apply the death-tick
  simulation-distance gate.
- `Entity.tick`/`baseTick`, `LivingEntity.baseTick`/`tick`, and
  `LivingEntity.calculateEntityAnimation` own base counters, interpolation
  inputs, hurt/death state, swimming, swing, and living walk animation. Client-
  only work must remain client-owned; server damage, AI, and authoritative
  movement remain excluded.
- `RemotePlayer.tick` and `aiStep` retain remote position/head/motion
  interpolation, swing, and bob behavior. Vehicle implementations select local
  authority before movement/serverbound reporting; remote vehicles consume
  interpolation instead.
- Entity subclass ticks such as guardian, wither, dragon, projectile, boat,
  minecart, evoker-fangs, and ominous-item-spawner paths produce client-visible
  animation, particle, or sound state. Generated class/interface/type catalogs,
  not a new approximate list, must select any specialized system.
- `ClientPacketListener.handleAddEntity`, `handleSetEntityMotion`,
  `handleSetEntityData`, `handleEntityPositionSync`, `handleTeleportEntity`,
  `handleMoveEntity`, and `handleRemoveEntities` all use the one packet
  processor owner. BBB may batch queries differently but must retain this
  mutation order relative to client ticks and publication.

## Initial Cost Ownership Hypotheses

The existing 2,000-zombie fixture measures about 83.904 ms for
`advance_entity_client_animations(1)` plus publication and about 0.905 ms for
publication alone. Inspection shows that one tick currently:

1. builds temporary water, spectator, projectile, trident, wither-target, and
   related maps or sets;
2. performs separate transform-interpolation and head-rotation queries;
3. performs a broad query over identity, transform, mount, metadata,
   attributes, optional equipment, and mutable animations for every entity;
4. computes many family-specific metadata projections unconditionally for each
   broad-query row; and
5. scans entity order again for minecart-step totals.

`EntityStore::clone` independently resolves each protocol id, borrows twelve
required components plus optional family components, spawns a new hecs entity,
and rebuilds the protocol-id map and order. These are hypotheses, not the final
attribution: phase 1 records tick-only, publication-only, combined, retained,
sparse/dense, churn, mutation-family, interpolation, and heterogeneous cases
before an implementation direction is frozen.

### Before Baseline

Command:

```text
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo bench -p bbb-world --bench publication_snapshot -- --noplot
```

Machine: AMD Ryzen 9 7945HX (16 cores/32 threads), Linux
6.8.0-124-generic, `rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)`.
Baseline commit: `daf81b15f34516484b69a52a0ca16a43e1d95ad0` plus the
measurement-only worktree changes in phase 1. Criterion uses ten samples, a
500 ms warm-up, and a two-second measurement window. These values are records,
not CI thresholds.

| Scenario | Before estimate |
| --- | ---: |
| empty tick-only | 28.471 ns |
| 100 zombies tick-only | 4.330 ms |
| 500 zombies tick-only | 21.602 ms |
| 2,000 zombies tick-only | 85.410 ms |
| 2,000 heterogeneous entities tick-only | 78.730 ms |
| empty tick + publication | 2.663 us |
| 100 zombies tick + publication | 4.370 ms |
| 500 zombies tick + publication | 21.845 ms |
| 2,000 zombies tick + publication | 88.226 ms |
| 2,000 heterogeneous entities tick + publication | 80.204 ms |
| 100 entities publication-only | 45.354 us |
| 500 entities publication-only | 222.55 us |
| 2,000 entities publication-only | 907.56 us |
| retained next publication, 2,000 entities | 978.84 us |
| transform + metadata on 8/2,000, then publication | 1.197 ms |
| metadata/equipment/effect/attribute/event, then publication | 1.071 ms |
| remove + respawn 16/2,000, then publication | 1.970 ms |
| 1,000 vehicles + 1,000 passengers interpolation tick + publication | 81.759 ms |
| 2,000 firework ticks + trail-effect drain | 75.608 ms |
| full `EntityState` projection, 2,000 heterogeneous entities | 1.422 ms |
| narrow transform projection, 2,000 heterogeneous entities | 77.322 us |
| 2,000 protocol-id lookups + narrow transform projection | 105.70 us |
| 1,000 protocol-id transform mutations in a 2,000-entity world | 10.626 ms |
| renderer model-source projection, 2,000 heterogeneous entities | 59.602 ms |

The 100/500/2,000 tick series is almost exactly linear, while publication is
about 1.0% of the 2,000-zombie combined path. Retaining the previous generation
adds only about 71 us to a full publication. Sparse and mutation-family cases
remain dominated by the full entity clone, and spawn/remove adds index/archetype
churn on top. The firework and heterogeneous cases are faster than homogeneous
zombies despite producing effects or exercising many families, so neither
effect-vector allocation nor one specialized family explains the dominant
cost. Full debug projection is also far below the tick cost and is not called
by the tick path. Renderer model extraction is independently expensive and must
not absorb simulation work as an apparent tick improvement.

The lookup microbench further bounds ordered-map plus single-component borrow
cost: 2,000 read lookups take about 106 us, while 1,000 `apply_entity_move`
updates take 10.626 ms because that path also resolves local authority,
feature-dependent interpolation, and supporting-block state. Neither reaches
the roughly 42.7 us per entity paid by the broad client-tick path. Linux sampling
profiling was attempted, but this machine sets `perf_event_paranoid=4`; the
phase therefore uses the reproducible microbench partitions and source-owned
query boundaries rather than privileged samples.

### Frozen Phase 2 Tick Contract

Source-level partitioning identifies one dominant, unrelated projection before
the mutable animation query: `wither_head_targets_by_id` currently resolves the
eye height of every entity into a temporary map on every client tick, even when
the world contains no Wither. Eye-height resolution enters the renderer and
dimension schema path, while the 2,000-entity renderer-model projection alone
measures 59.602 ms. This is separate from hecs query/borrow, protocol-id lookup,
the animation helper, and effect-vector costs measured above.

Phase 2 therefore freezes this first implementation boundary:

- generated `VANILLA_ENTITY_TYPE_WITHER_ID` membership selects only Withers;
- the first narrow query gathers their transform, scale, and generated metadata
  target ids without projecting any unrelated entity;
- only distinct positive side-head target ids use the existing protocol-id to
  hecs handle index, and only those targets borrow the components required to
  derive eye height;
- no-Wither worlds allocate neither the old eye-position map nor target
  projections, while Wither fallback rotations and target order remain exact;
- the generated firework metadata accessor remains the trail membership owner;
  its catalog guard explicitly rejects a direct type-constant shortcut; and
- interpolation, head lerp, broad animation mutation, client effect append,
  minecart lerp, and publication retain their existing relative order.

The same baseline scenarios are rerun immediately after this change. Any
additional schema lookup index, family query, or tick-state cache is permitted
only when that rerun shows a residual material owner; such data must be derived
from the generated catalog and may not become a parallel membership/schema
table. Entity publication storage remains unchanged in phase 2 so its measured
cost cannot be hidden by the tick optimization.

The first rerun reduced the 2,000-zombie tick from 85.410 ms to 67.298 ms, but
a new direct partition measured 17 representative generated accessor lookups
for 2,000 entities at 46.802 ms for zombies and 44.523 ms for a heterogeneous
mix. The generic resolver linearly scans the 157-entry type catalog, the full
accessor catalog, and the class catalog for every family helper. This freezes a
second evidence-backed change: directly index the generated contiguous
entity-type catalog, binary-search the generated class ordering, and narrow an
accessor search to its generated owner range. Tests guard the generated
continuity and ordering that these lookups require. No cached membership,
parallel schema, or per-entity copy is introduced.

That ordering-only change reduced the 17-accessor partition to 22.082 ms but
left it at about 61% of the resulting 36.293 ms zombie tick. The final lookup
step may therefore build one process-static, reference-only derived index on
first use. Each entity-type slot contains only references to the nearest
generated accessor entries in its inheritance chain; it duplicates no data id,
default, serializer, schema field, or authored membership. Exhaustive tests
compare the indexed result against a direct generated-chain walk for every
entity type and accessor name. The one-time allocation and first-use latency
remain visible as a startup/peak-memory tradeoff, not a per-generation owner.

### Phase 2 Tick Result

The same Criterion configuration and machine produced these phase-2 estimates.
Publication storage remains the phase-1 implementation in this comparison.

| Scenario | Before | Phase 2 | Improvement |
| --- | ---: | ---: | ---: |
| 2,000 zombies tick-only | 85.410 ms | 8.237 ms | 10.37x |
| 2,000 zombies tick + publication | 88.226 ms | 9.308 ms | 9.48x |
| 2,000 heterogeneous tick-only | 78.730 ms | 9.064 ms | 8.69x |
| 2,000 heterogeneous tick + publication | 80.204 ms | 10.234 ms | 7.84x |
| 1,000 vehicles + 1,000 passengers tick + publication | 81.759 ms | 9.158 ms | 8.93x |
| 2,000 firework ticks + trail-effect drain | 75.608 ms | 8.944 ms | 8.45x |
| 17 accessor lookups x 2,000 zombies | 46.802 ms | 0.217 ms | 215.7x |

The empty path remains about 28 ns, while the entity count series remains
linear (407 us/2.034 ms/8.237 ms for 100/500/2,000 zombies). The retained and
mutation publication cases are unchanged by construction, so the improvement
is not deferred into a later mutation. The derived index holds one boxed slice
per generated entity type and only pointer-sized references; a conservative
upper bound is 157 x 227 references (about 279 KiB on a 64-bit target) plus 157
box descriptors, while actual inheritance contains far fewer entries. It is
process-static and does not multiply with worlds or retained generations.

### Phase 3 Entity Payload Sharing

The phase-1 zombie fixture owns mostly empty component containers, so it bounds
the hecs shell rebuild but not non-empty payload cloning. A payload-heavy
partition fills all 2,000 entities with eight metadata values, six equipment
slots, an attribute, and three mob effects. Before structural sharing, unchanged
publication measured 22.955 ms and mutating metadata on 8/2,000 while retaining
the previous generation measured 22.987 ms.

The live hecs world remains the only mutable canonical entity owner. Its
metadata/painting projection, equipment, attribute, passenger, effect, damage,
and minecart-step containers now use the same serde-transparent
`PublicationShared<T>` owner as other world payloads. An `EntityStore` snapshot
still rebuilds its hecs entities and protocol-id handles, but clones only the
small component shells and payload Arcs. A mutation reaches
`PublicationShared::deref_mut`, which calls `Arc::make_mut` for that exact field;
transform and animation components remain direct values so a dense client tick
does not COW a whole entity or heavy component family.

On the same machine:

| Scenario | Before sharing | Phase 3 | Improvement |
| --- | ---: | ---: | ---: |
| payload-heavy unchanged publication, 2,000 | 22.955 ms | 1.003 ms | 22.90x |
| payload-heavy sparse metadata + publication, retained generation | 22.987 ms | 1.033 ms | 22.26x |
| default sparse transform/metadata + publication | 1.197 ms | 1.003 ms | 1.19x |
| metadata/equipment/effect/attribute/event + publication | 1.071 ms | 1.018 ms | 1.05x |
| spawn/remove 16/2,000 + publication | 1.970 ms | 1.962 ms | unchanged |
| 2,000 zombies tick-only | 8.237 ms | 8.025 ms | no regression |
| 2,000 zombies tick + publication | 9.308 ms | 9.135 ms | no regression |

Both sparse cases mutate while the benchmark retains the preceding snapshot,
and the tick-plus-publication benchmark likewise retains each preceding
generation, so the COW cost is included rather than deferred. Full owned
`EntityState` and renderer/debug projections still materialize their requested
Vec/map values at the existing API boundary; the serialization and probe shape
is unchanged. The remaining roughly 1 ms O(n) hecs shell rebuild is about 11%
of the representative zombie tick-plus-publication path and is no longer the
dominant end-to-end owner. It remains the next boundary for publication-only or
very sparse, empty-payload workloads; eliminating it would require a separate
immutable reader representation and query migration, which would add more risk
than the measured residual justifies in this slice.

Unchanged heavy payloads are shared across any number of retained generations.
If every entity mutates a heavy field while an older generation is retained,
one new copy of each changed field is necessarily live; retaining several
generations that each mutate the same payload can retain one version per
generation. No world-wide payload copy occurs for a sparse mutation. The
isolation matrix covers transform/rotation, metadata, equipment,
effects/attributes, animation counters, passenger/leash/vehicle state,
spawn/remove, entity event, and client-tick mutation, and also checks that a
metadata mutation breaks only its corresponding Arc owner.

### Phase 4 Runtime Observation And Final Rerun

The existing generation-matched `NetCounters` view now separates the three
owners that matter operationally:

- `simulation_entity_client_tick_nanos` and its process peak measure one
  non-empty due client-tick batch (one to ten catch-up ticks), excluding
  publication;
- `simulation_entity_client_tick_scanned_entities` and
  `simulation_entity_client_tick_mutated_entities` count tracked candidates
  and ordered root/passenger mutations across that batch. Every scheduled row
  advances `age_ticks`, so the mutation count is exact without comparing the
  large animation component;
- `simulation_entity_client_tick_{preparation,schedule,projectile,interpolation,animation}_nanos`
  and their process peaks split that batch into the world-input, ordering,
  projectile, transform, and family-animation owners. These low-frequency
  counters exposed the former per-entity Java class-graph interpolation lookup
  and verify its generated-catalog replacement;
- `simulation_entity_snapshot_build_nanos` and its process peak measure only
  the `EntityStore::clone` hecs-shell rebuild, while
  `simulation_entity_snapshot_rebuilt_entities` counts successfully rebuilt
  entities; and
- the pre-existing publication duration continues to cover the complete outer
  generation build. All values enter the same immutable publication and exact
  world `Arc` consumed by render and control.

The instrumentation adds no log, allocator hook, cache, or reader copy. A
simulation step with no due client tick retains the latest real tick sample
instead of replacing it with scheduler bookkeeping. The snapshot timer is one
`Instant` pair per entity-store clone; logical counts reuse the existing query
and clone loops.

The final full Criterion rerun includes this observation overhead. It used the
same machine and configuration as the before baseline.

| Scenario | Before | Final | Improvement |
| --- | ---: | ---: | ---: |
| 2,000 zombies tick-only | 85.410 ms | 8.141 ms | 10.49x |
| 2,000 zombies tick + publication | 88.226 ms | 9.304 ms | 9.48x |
| 2,000 heterogeneous tick-only | 78.730 ms | 8.892 ms | 8.85x |
| 2,000 heterogeneous tick + publication | 80.204 ms | 10.175 ms | 7.88x |
| retained next publication, 2,000 entities | 0.979 ms | 0.944 ms | 1.04x |
| payload-heavy unchanged publication, 2,000 | 22.955 ms | 0.966 ms | 23.77x |
| payload-heavy sparse metadata + retained publication | 22.987 ms | 1.010 ms | 22.77x |
| 1,000 vehicles + 1,000 passengers tick + publication | 81.759 ms | 9.063 ms | 9.02x |
| 2,000 firework ticks + trail-effect drain | 75.608 ms | 8.862 ms | 8.53x |
| 17 accessor lookups x 2,000 zombies | 46.802 ms | 0.214 ms | 218.7x |
| renderer model-source projection, 2,000 heterogeneous | 59.602 ms | 4.939 ms | 12.07x |

The 100/500/2,000 zombie tick series remains linear at about 0.402/2.020/8.141
ms. The approximately 0.94 ms entity-shell rebuild is now about 10% of the
representative tick-plus-publication path and is not its dominant owner.
Eliminating that shell would require a second immutable reader representation
and migration of every render/control/probe query; measurement does not justify
that architecture in this slice. Renderer model projection became cheaper from
the same generated schema index rather than absorbing simulation work. Full
owned debug projection remains an explicit, independently measured 1.53 ms API
boundary and is not invoked by the client-tick path.

Peak memory remains proportional to live hecs shells for retained generations,
plus one payload version per retained generation that observed a later-mutated
heavy field. Sparse mutation does not duplicate unrelated payloads; dense
metadata/equipment/effect updates can temporarily retain every changed field.
The process-static generated accessor reference index has the bounded footprint
recorded in phase 2. No reliable byte count is reported because doing so would
require allocator instrumentation outside the no-`unsafe` contract.

## Serial Phases And Checkpoints

1. **Reproducible baseline and attribution surface.** Extend the existing
   repo-native Criterion target with empty/100/500/2,000 entity counts and the
   required tick, publication, retained, sparse/dense, churn, mutation-family,
   passenger/interpolation, and heterogeneous scenarios. Record same-machine
   before data without CI wall-clock thresholds. Focused validation:
   `cargo bench -p bbb-world --bench publication_snapshot --no-run`, the full
   benchmark, `cargo test -p bbb-world`, format/diff/doc checks. Commit a clean,
   reviewable measurement target and run a full gate. This gated tip is the
   first safe stop.
2. **Measured entity-tick optimization.** Add low-overhead logical phase/work
   accounting, then replace only the measured dominant broad scans, repeated
   lookups, or temporary projections with narrow hecs queries and generated
   catalog/schema selection. Preserve tick/effect order with focused world and
   native semantic tests. Rerun the benchmark, commit a buildable state, and run
   a full gate because this changes core simulation behavior. This gated tip is
   a safe stop.
3. **Measured immutable entity generation.** If publication remains material,
   implement the smallest per-entity or entity-page structural generation whose
   sparse mutation plus publication is cheaper end to end. Audit every mutation,
   removal, id reuse, clear/respawn/session reset, serde/probe projection, dense
   update, and retained-generation peak. Add the complete old-generation
   isolation matrix and same-Arc/generation regressions. Focused validation:
   `bbb-world`, affected `bbb-native` publication/thread tests, benchmark, and
   format/diff/doc checks; commit and run a full gate. If measurement instead
   proves full ECS publication immaterial, record that boundary and omit this
   storage change rather than adding unneeded COW.
4. **Runtime observation and closeout.** Publish generation-matched recent/peak
   entity tick and entity snapshot durations plus scanned/mutated/rebuilt counts
   through the existing counters owner. Verify effect ordering, blocked-render
   deadlines/network wake, shutdown/join, panic propagation, fixture/probe path,
   and control/render pairing. Update architecture, unsupported ledger, archive,
   before/after data, residual hotspots, and retained-memory risks. Run affected
   focused tests, final full gate, and `scripts/playable-gate.sh auto` at the
   delivered `HEAD`.

No milestone may contain more than four normal commits before its required full
gate. A failed focused test or benchmark anomaly forces immediate diagnosis and
a normal fix-forward commit; history is never rewritten.

## Failure Localization And Safe Stops

Phase 1 changes measurement only, phase 2 changes tick query/behavior ownership,
phase 3 changes immutable entity storage, and phase 4 changes observation and
integration. This separation makes a semantic regression attributable to one
boundary. Each successful milestone gate is a safe stop. Once a storage
migration starts, the slice stops only after one complete publication path is
restored and gated; live deep-copy and structural paths may not remain runtime-
selectable. Missing Java may block only the live portion of the final playable
gate; the recorded fixture smoke still runs and the residual reason is reported.
