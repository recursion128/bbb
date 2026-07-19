# Entity Tick And Publication Cost

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
