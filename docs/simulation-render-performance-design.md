# Simulation / Publication / Render Performance

This document is the pre-start contract and measurement record for the
high-risk engineering slice that removes render-driven simulation work,
projects only visible entities, reuses render extraction state, and makes
terrain GPU residency incremental. It refines the ownership contracts in
[`simulation-render-thread-design.md`](simulation-render-thread-design.md),
[`simulation-publication-snapshot-design.md`](simulation-publication-snapshot-design.md),
and [`entity-tick-publication-cost-design.md`](entity-tick-publication-cost-design.md).

## Semantic Goal And Completion

The named simulation thread remains the only mutable `WorldStore` and input
owner. Render and control continue to consume one immutable publication
generation and the same exact world `Arc`. At equal machine, GPU, resolution,
view distance, entity distance, graphics, particles, resource packs, camera
path, and actually visible content, stable bbb CPU frame p50 and p95 should beat
the official vanilla Java 26.1 client without degrading p99, client-tick
deadlines, network response, or visible output.

Completion requires all of the following:

- render FPS no longer drives simulation steps, effect batches, publications,
  outputs, or winit wakes;
- a true no-op scheduler wake advances no generation or effect sequence and
  builds no world/entity snapshot;
- input/diagnostic-only generations reuse the previous exact world `Arc`;
- entity extraction performs one narrow candidate query and vanilla-equivalent
  visibility filtering before one batch full projection of survivors;
- render-owned extraction storage clears and reuses capacity, and publication
  carries a narrow render input instead of cloning complete input/frame heaps;
- a changed terrain owner replaces only its own GPU vertex/index residency;
  ordinary block/chunk changes never clear or re-upload unchanged owners;
- background meshing is added only if the headless and live measurements show a
  material CPU-frame or tail-latency win; any worker receives immutable values,
  has a bounded prioritized queue, and discards stale revisions; and
- same-machine before/after and strict official comparison data are recorded.

## Non-Goals And Hard Invariants

This slice changes no protocol, client tick cadence, input semantics,
interpolation, render result, effect order, server-authority boundary, view
distance, entity distance, particle count, graphics quality, or diagnostic
content. It introduces no shared mutable world, second canonical entity/world,
control copy, skipped necessary publication, mixed generation, global
allocator, `unsafe`, configuration UI, Rayon, or unmeasured concurrent ECS
owner. The existing 49-chunk renderer selection is not accepted as an advantage
over an official client rendering more terrain.

The effect-before-publication gate, connection FIFO, blocked-render isolation,
single simulation owner source guards, shutdown/join, panic propagation, and
online/probe/fixture deterministic side-effect path remain mandatory.

## Owners And Frozen Integration Contract

- `bbb-native::simulation` owns scheduler wake classification, latest-value
  render telemetry, effect/publication sequencing, no-op suppression, world-Arc
  reuse, and generation-matched observations.
- `bbb-world` owns canonical revisions and narrow entity candidate/full survivor
  projection. New membership facts derive from the generated type/schema
  catalogs; no parallel type table or metadata schema is permitted.
- `bbb-native` render extraction owns persistent scratch/caches, culling order,
  frame phase measurements, and terrain CPU cache/job scheduling. A cache key
  includes every level/content/resource revision needed to reject stale data.
- `bbb-renderer` owns GPU terrain handles and replaces/removes one stable
  chunk/section/layer owner at a time. Wgpu resources never cross to simulation
  or a mesh worker.
- `bbb-control` exposes only the same generation's low-cost logical/time
  observations. It receives neither a world nor entity copy.

`TerrainReadiness` is an edge-triggered level-generation value. Backend
counters are latest-value telemetry: render writes replace the pending value
without waking simulation, and the next real network/input/tick step absorbs
it. A final value remains available until observed. Winit output wake uses one
pending flag so multiple queued outputs need at most one outstanding wake; the
consumer clears it only after draining and closes the producer race by
rechecking the queue.

A world snapshot is rebuilt only when canonical world state changed since the
last publication. A publication needed for input, counters, clocks, or
diagnostics can reuse the previous world `Arc`; render and control publish that
same Arc/generation pair. A no-op produces neither publication nor output. A
non-empty effect batch is visible before the first publication carrying its
sequence; an empty collector consumes no sequence.

Entity extraction first emits a narrow value containing interpolated transform,
bounds, identity, section position, and only flags needed for the vanilla
visibility decision. It then applies renderer distance/frustum, compiled and
visible section, camera/local-player, indirect-passenger, spectator/sleeping/
detached-camera, and large/custom bounds rules in vanilla order. Survivors are
full-projected once in batch; relationship consumers reuse the candidate index
instead of scanning all targets per survivor.

Terrain residency keys one GPU owner by level generation, chunk/section,
render layer, mesh revision, and resource/texture revision. Replacement uploads
only changed vertex/index bytes; disappearance removes only that owner.
Translucent resort uploads a new index buffer without retransmitting unchanged
vertices whenever its mesh revision is unchanged.

## Vanilla 26.1 Evidence

Local vanilla sources define the behavior rather than implementation memory:

- `LevelRenderer.extractVisibleEntities` calls dispatcher distance/frustum
  culling (or the indirect-passenger exception), checks compiled/visible
  section, applies camera/detached/sleeping and LocalPlayer rules, and only then
  calls `extractEntity` to build `EntityRenderState`.
- `LevelRenderState.reset` and `GuiRenderState.reset` clear persistent lists and
  sets for reuse each frame instead of replacing the state owners.
- `Minecraft.runTick` processes packets/tasks and fixed client ticks before
  `renderFrame`; render cadence is not a source of extra logical ticks.
- `SectionRenderDispatcher.RenderSection` owns dirty/player-changed state and a
  stable compiled section mesh. Rebuild uploads allocations for the changed
  section; transparency resort supplies only a new index buffer.
- `ClientPacketListener` still serializes all canonical level/entity mutation
  through the packet processor, matching bbb's one simulation owner.

## Before Attribution

Baseline HEAD is `a9ba20a7a` on an AMD Ryzen 9 7945HX (16 cores/32 threads),
Linux 6.8.0-124-generic, `rustc 1.96.0-nightly
(bcf3d36c9 2026-03-19)`. Criterion uses the checked-in configuration; values
are records, never CI wall-clock thresholds.

Commands:

```text
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo bench -p bbb-world --bench entity_projection -- --noplot
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo bench -p bbb-world --bench publication_snapshot -- --noplot
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo bench -p bbb-renderer --bench terrain_residency -- --noplot
```

Existing before measurements:

| Owner / scenario | Before estimate |
| --- | ---: |
| full entity model-source projection, 100 | 272.10 us |
| full entity model-source projection, 500 | 1.430 ms |
| full entity model-source projection, 2,000 | 5.798 ms |
| entity pick-target projection, 2,000 | 1.219 ms |
| narrow transform projection, 2,000 heterogeneous | 82.917 us |
| current full projection then 0% visible, 2,000 | 5.743 ms |
| current full projection then 10% visible, 2,000 | 5.729 ms |
| current full projection then 50% visible, 2,000 | 5.715 ms |
| current full projection then 100% visible, 2,000 | 5.783 ms |
| unchanged entity publication, 2,000 | 0.903 ms |
| entity tick + publication, 2,000 heterogeneous | 10.179 ms |
| unchanged 625-column publication | 10.544 us |
| single block mutation + 625-column publication | 20.579 us |
| terrain CPU snapshot materialization, 1 / 8 / 32 owners | 0.143 / 9.190 / 40.759 ms |
| terrain CPU mesh build, 1 / 8 / 32 changed owners | 1.855 / 18.376 / 77.536 ms |
| translucent camera-dependent mesh input, 1 owner | 1.849 ms |

Source and deterministic test attribution is unambiguous before optimization:

- every render frame enqueues `TerrainReadiness` and `BackendCounters`; below
  the mailbox threshold neither latest-value command coalesces, so render
  cadence wakes simulation;
- every simulation scheduler pass allocates an effect sequence, seals an empty
  batch, rebuilds the publication, sends output, and wakes winit;
- the entity model path full-projects every entity before native removes the
  local/camera entity, so its cost is almost invariant at 0%, 10%, 50%, or 100%
  survivors;
- `RendererFrame` materializes many owned vectors, clones complete
  `ClientInputState`, and clones heap-backed `SimulationFrameState` fields;
- one changed CPU terrain mesh calls the renderer's full residency replacement,
  clearing all three layer vectors and recreating every resident GPU buffer.

The new measurement targets record the current 0/10/50/100% projection order,
terrain CPU snapshot materialization, 1/8/32 changed-owner mesh builds, and a
camera-dependent translucent mesh input. GPU upload and full-frame numbers are
not claimed from this headless target.

## Runtime Observation Contract

Low-overhead counters published with one immutable generation will report:

- productive/no-op steps, publications, world snapshot rebuild/reuse, effect
  batches and empty suppression, render wakes sent/coalesced, output backlog;
- entity candidates/survivors/full projections plus recent/peak cull,
  projection, model-build, and major extraction-phase durations;
- terrain jobs queued/completed/stale, changed/uploaded owners, vertex/index
  bytes, full replacement count, resident logical bytes, and recent/peak work;
  and
- render scratch capacity growth/reallocation counts for the major reusable
  vectors. No high-frequency log or allocator hook is introduced.

## Official Comparison Protocol

Both clients use the same machine/GPU, resolution, view and simulation
distance, entity distance, graphics mode, particles, mip level, clouds,
resource packs, vsync/FPS cap, offline server fixture/world seed, entity and
particle spawn script, and replayed camera path. The record includes actual
visible chunks/sections/entities so bbb's current resident cap cannot reduce
the workload silently.

Each client warms up before at least three fixed-duration captures. Report CPU
frame p50/p95/p99, GPU frame, 1% low, chunk-update spike, tick deadline misses,
network response latency, and changed/uploaded terrain owners/bytes. Record
Java executable/version and launch arguments, bbb commit/arguments, GPU driver,
profiler/capture tool, fixture revision, and raw output path. Compare only
end-to-end frame/tail metrics collected by equivalent external instrumentation;
do not compare internal section timers from different profilers as if they were
the same quantity.

This host has the fixed Java 25 runtime and vanilla 26.1 server jar required by
`scripts/playable-gate.sh live`. Those paths validate bbb's live session; a
strict official-client CPU comparison still requires a reproducible official
client launch/capture with the equal-load settings above and remains phase 6,
not fabricated phase-1 data.

## Phase 2 Boundary Result

The first runtime milestone removes the render feedback loop without adding a
world dirty bit. Readiness is generation-scoped and edge-triggered; backend
telemetry is one non-waking latest-value slot; output wakes are coalesced behind
one pending flag; and an early wake with no command or network value emits no
effect, generation, output, or replacement deadline. Empty effect collectors
do not consume sequence space. Runtime observations now include productive and
no-op steps, publication and world rebuild/reuse counts, non-empty/empty-
suppressed effect counts, output backlog, and sent/coalesced render wakes in the
next immutable generation.

World reuse is intentionally conservative. A due tick or any pending network
event rebuilds. The exhaustive command classifier rebuilds for gameplay input,
control, readiness/load, and reload work; only surface/text-measurement and
platform result commands plus shutdown can reuse. The reuse path publishes the
previous exact world `Arc` to both render and control and reports zero entity
snapshot time/rebuilt rows.

Same-host Criterion after measurement:

| World portion / scenario | Before rebuild | Exact Arc reuse | Reduction |
| --- | ---: | ---: | ---: |
| 2,000 entity unchanged world value | 914.69 us | 2.360 ns | about 387,600x |

The 2.360 ns measurement covers only retaining/releasing the already immutable
world owner, matching the eliminated `WorldStore::publication_snapshot`
portion. It does not claim that the complete publication is 2.360 ns:
`ClientInputState` and heap-backed `SimulationFrameState` work remain phase-3
owners and are deliberately still visible in
`simulation_publication_build_nanos`. The scheduler/backend tests are logical
count assertions rather than unstable wall-clock thresholds: repeated backend
frames leave both command length and wake generation unchanged; a synthetic
producer wake before the deadline creates no output; blocked-render output
creates one sent wake and coalesces later outputs while deadlines and network
wakes continue.

## Phase 3 Boundary Result

Entity rendering now fills a retained narrow hecs batch of identity, current
transform, dynamic model bounds, mount/leash, spectator/sleeping, fishing-owner,
and generated renderer-culling facts. Native applies vanilla's dispatcher,
compiled-terrain, camera/local-player, passenger, leash and renderer-class
rules before a single full projection of survivors. The complete candidate set
remains available to relationship logic, while the established public full
projection preserves protocol order. Candidate/survivor/id-index/source/final-
instance allocations are reused; applying a frame swaps the renderer's previous
entity-instance allocation back to extraction scratch.

Publication now carries explicit `RenderInputPublication`. It copies only the
keys/buttons and screen/debug state read by render/event-loop code, not gameplay
movement accumulators, click timers, outbound request queues, query/crash state,
or other simulation-only heaps. Render borrows `SimulationFrameState` instead
of cloning its particle context vectors. Conditional command-suggestion and
stats-screen projection may clone only this already-narrow display state.

Same-host Criterion after data for 2,000 heterogeneous entities:

| Full projection survivors | Before: project all then filter | Visibility first | Reduction |
| --- | ---: | ---: | ---: |
| 0% | 5.743 ms | 1.267 ms | 4.53x |
| 10% | 5.729 ms | 1.676 ms | 3.42x |
| 50% | 5.715 ms | 3.299 ms | 1.73x |
| 100% | 5.783 ms | 5.338 ms | 1.08x |

The retained narrow candidate batch alone is 60.814 us / 309.39 us / 1.238 ms
for 100 / 500 / 2,000 entities. Its remaining cost is dominated by exact
dynamic model/culling bounds rather than the 82.917 us transform-only hecs
query. Rare Guardian/EndCrystal/Shulker/display specialized culling stays
conservative when the narrow row lacks its renderer-owned beam/interpolation
box; it may over-project but cannot hide visible content. Section-granular
compiled visibility and the remaining non-entity `RendererFrame` vector owners
remain measured phase-4/final tails rather than being claimed complete here.

## Phase 4 Boundary Result

Terrain CPU meshes are now partitioned into independently resident section and
render-layer values while retaining the same chunk-neighborhood lookup for
occlusion, lighting, fluid overlays, and biome-derived material input. The
render owner keys residency by client-level generation, chunk, section, and
layer. A monotonic native resident revision changes on every rebuilt chunk,
including resource reload, while a level-generation transition clears the old
CPU cache without reusing its GPU identity.

`bbb-renderer` retains the exact vertex/index buffers of every unchanged chunk.
One changed chunk removes and recreates only its non-empty section/layer owners;
load/unload is a desired-owner set difference. The legacy full replacement API
remains for explicit callers and increments a dedicated counter, but the native
terrain path no longer calls it. Camera-dependent translucent sorting retains
vertices and writes only each changed section's index buffer. Renderer telemetry
now distinguishes changed chunks, uploaded/removed section-layer owners,
vertex/index bytes, translucent resort owners/index bytes, explicit full
replacements, and current/logical-peak resident bytes.

The offscreen GPU regression uses two resident chunks. An unchanged sync reports
zero changed/uploaded/removed owners and zero upload bytes; changing one chunk
reports exactly one changed chunk and one uploaded/removed non-empty owner while
the second stays resident; unloading then removes only the named owner. Reusing
the same numeric revision in a new level generation still replaces the owner.
The section partition test proves that summed vertices, indices, faces, and
source-section count match the former column mesh.

Same-host Criterion after section partitioning:

| Terrain CPU scenario | Before column mesh | Section mesh | Change |
| --- | ---: | ---: | ---: |
| 1 changed chunk | 1.855 ms | 1.850 ms | no material change |
| 8 changed chunks | 18.376 ms | 18.479 ms | no material change |
| 32 changed chunks | 77.536 ms | 70.985 ms | 1.09x faster in this run |
| translucent build input, 1 chunk | 1.849 ms | 1.909 ms | 3.1% slower |

The last row rebuilds the whole CPU mesh to preserve the historical Criterion
comparison; production camera movement does not take that path and uses the new
index-only resident resort. GPU buffer creation has no unstable wall-clock CI
threshold: the exact logical upload bytes and owner counts are asserted instead,
and the final live capture owns end-to-end GPU timing. Wgpu may defer destruction
of a replaced buffer, so `terrain_peak_resident_bytes` is a logical resident
peak rather than a claim about short-lived driver allocation peaks.

## Phase 5 Boundary Result

The measured synchronous residual justified one bounded terrain mesh worker:
one representative changed chunk costs about 1.85 ms, which is 27% of a
144-FPS CPU frame budget, while a 32-chunk burst contains about 71 ms of mesh
work. The native render owner now converts canonical world data into immutable
`Arc<TerrainChunkSnapshot>` values and lazily starts one named
`bbb-terrain-mesh` thread. A job contains only the level/mesh/content key,
target and neighbor snapshot Arcs, an immutable atlas Arc, camera position, and
priority. It contains no `WorldStore`, renderer, wgpu object, or mutable
canonical owner.

The priority queue holds at most four jobs, first prefers chunks with an active
canonical local block prediction, then the nearest selected chunk (the
player/camera chunk has distance zero). A newer queued revision for
the same chunk replaces the old entry; a nearer job may evict the farthest
queued entry. The result channel holds at most five meshes—the queue plus the
single in-flight job—so a blocked render thread cannot create unbounded mesh
memory. Every accepted result must still match client-level generation,
resource/mesh generation, chunk mesh revision, desired selection, and retained
snapshot owner. Reload/reset cancels queued values, and in-flight stale values
are counted and discarded. GPU creation/upload remains render-thread-only.

Renderer telemetry now publishes cumulative queued/completed/stale/queue-full
jobs, current/peak queue depth, and recent/peak worker build nanoseconds with
the next immutable generation. The worker propagates a panic on the next render
interaction, stops through its condition variable, and is always joined by the
terrain upload owner's drop path. Focused tests cover bounded priority and
same-owner replacement, immutable source ownership, build/result delivery,
join, stale-result rejection, reset identity, and counter projection. The
historical 1/8/32 Criterion remains the amount of CPU mesh work rather than a
claim that work disappeared; final live frame/tail capture owns the end-to-end
benefit. Snapshot extraction and section-dirty granularity remain render-thread
costs recorded as follow-up boundaries.

## Serial Phases, Validation, And Gates

1. **Baseline, contract, observation surface.** Add the headless Criterion
   partitions above and freeze this document. Focused validation: both world
   benches, renderer bench, benchmark `--no-run`, relevant crate tests, format,
   diff, docs lint, and full gate. This gated tip is the first safe stop.
2. **Wake/no-op/publication reuse.** Implement edge readiness, non-waking latest
   telemetry, pending winit wake, empty-effect suppression, no-op output
   suppression, and exact world-Arc reuse. Focused simulation publication,
   scheduler, ordering, blocked-render, shutdown, and control pairing tests;
   rerun boundary measurements and full gate.
3. **Visibility-first entity extraction and persistent scratch.** Add narrow
   candidates, exact culling exceptions, survivor batch projection, narrow
   render input, persistent extraction storage/caches, and phase observations.
   Focused world/native/renderer semantic equivalence and benchmark tests, then
   full gate.
4. **Incremental terrain residency.** Add stable owner handles/revisions,
   changed-owner upload/removal, index-only translucent resort, counters, and
   reset/reload tests. Rerun 1/8/32 and live GPU captures, then full gate.
5. **Measured background meshing, if justified.** Only a material residual mesh
   cost enables an immutable, bounded, revision-checked worker. Otherwise record
   the no-go decision and make no concurrency change. Any worker milestone gets
   its own focused lifecycle/stale-result tests and full gate.
6. **Official comparison and closeout.** Record before/after/official runs,
   update architecture/ledger/archive with remaining CPU/GPU/memory risks, and
   run the final delivery sequence at the same committed HEAD: full gate,
   `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo build -p bbb-native`, verify the
   resulting binary timestamp is newer than that HEAD, then run the required
   Weston-backed `scripts/playable-gate.sh live`. The live run must use the
   configured local Java 25 runtime, vanilla 26.1 server jar, and `bbb-gate`
   Wayland socket, and must complete the status/login/configuration/play,
   control, movement, and block-interaction phases. Run
   `scripts/playable-gate.sh auto` afterwards when project policy still
   requires it; fixture or auto fallback never substitutes for the explicit
   live result.

No milestone contains more than four normal commits before a full gate. Every
phase commit is buildable and receives affected focused tests, fmt, diff check,
docs lint when applicable, and complete diff review.

## Failure Localization And Safe Stops

Phase 1 changes measurement only; phase 2 owns cross-thread scheduling and
publication; phase 3 owns entity/render extraction; phase 4 owns GPU terrain;
phase 5, if enabled, owns asynchronous lifecycle. Failures therefore localize
to one boundary. Fixes are normal forward commits; history is never rewritten.
Once a runtime owner migration starts, a safe stop exists only at its successful
full-gated milestone—dual selectable paths, mixed generations, or partially
incremental terrain residency are not deliverable states. Missing Java/GPU
fixture can block only the final live comparison, not the repo-native phases.
The current host already has both a live Weston process and
`/run/user/1000/bbb-gate`; final validation rechecks both rather than trusting a
stale socket. A live failure records its artifact directory and is classified
as environment, stale binary, client exit, control disconnect, timeout, or
gameplay assertion before any code conclusion. Fix-forward runs rebuild the
same delivery HEAD and rerun the complete live gate. Headless synthetic input
keeps the existing `input.focus {"focused": true}` command ahead of look,
keyboard, and mouse input.
