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

## Non-entity render extraction milestone contract (2026-07-19)

This high-risk engineering milestone closes the remaining heap-heavy
non-entity `RendererFrame` path before selecting another visible feature. Its
semantic goal is to reduce steady and tail CPU time across native extraction,
renderer apply, and GPU residency while preserving the exact published world,
frame ordering, and visible submissions. Completion requires all of the
following:

- the HUD/item/map/block-entity/debug extraction phases have repo-native
  before/after measurements and low-overhead recent/peak observations;
- reusable frame owners report logical capacity growth without an allocator
  hook, and steady frames retain their established capacity;
- stable item/map/HUD projections use exact semantic keys or exact output
  equality before reuse; a revision may reject a cache entry quickly but a
  hash alone never establishes a hit;
- unchanged item/map render owners do not repeat CPU geometry merge, transient
  atlas construction, or GPU vertex/index upload, while changed owners replace
  only their own residency; and
- level reset, resource reload, registry revision, entity-id reuse, inventory
  mutation, and debug-mode changes cannot select stale render data. Dynamic
  partial-tick, camera, wall-clock, animation, hover/fade, and particle-derived
  values remain frame-local.

The milestone does not change protocol, tick cadence, simulation/publication
ownership, visibility, draw ordering, item transforms, GUI behavior, resource
semantics, or the official-comparison workload. It does not add a canonical
world, renderer-to-simulation mutable sharing, an allocator hook, `unsafe`,
parallel extraction, or a configuration UI. Reusing an outer `Vec` without
removing its downstream merge/upload cost is not sufficient.

Ownership stays explicit across `bbb-world` (immutable canonical generation
and registry revisions), `bbb-native` (render-thread extraction scratch, cache
keys, phase observation, and one `RendererFrame` commit), `bbb-renderer`
(derived CPU/GPU owner residency and exact-output replacement), and
`bbb-control` (serde-defaulted immutable diagnostic projection). Vanilla 26.1
evidence is `GameRenderer.extract`/`extractGui`,
`LevelRenderer.extractLevel`, `LevelRenderState.reset`, and
`GuiRenderState.reset`: the official client retains render-state containers,
clears their lists before extraction, and keeps extraction before submission.
Native therefore may retain capacity and unchanged derived GPU owners, but it
must clear logical contents and preserve the same extraction/submission order.

The serial phases and failure-localization boundaries are:

1. add the repo-native baseline and phase/capacity/residency observations;
   focused renderer/native/control tests plus a milestone full gate establish
   the unoptimized checkpoint;
2. retain native frame storage and remove redundant registry/string and
   aggregation owners; focused extraction equality, invalidation, and retained
   capacity tests localize native ownership failures;
3. retain exact unchanged renderer item/map CPU/GPU owners and replace only
   changed owners; offscreen renderer tests localize upload, atlas, and draw
   regressions;
4. cover reset/reload/level/entity/inventory/debug invalidation, rerun the
   identical benchmark matrix, update the ledger/archive, and run the final
   full gate;
5. build `bbb-native` from the delivered HEAD, verify its timestamp, then run
   the explicit live Vanilla 26.1 playable gate and the policy-required auto
   gate.

The phase-1 same-host Criterion baseline uses
`frame_extraction_storage` (20 samples, 500 ms warm-up, two-second
measurement). It isolates the repeated owned materialization before any wgpu
work; runtime phase counters and offscreen tests cover the downstream boundary.

| Fresh extraction owner workload | Baseline mean |
| --- | ---: |
| 19 item-model buckets, empty | 0.748 us |
| 19 item-model buckets, one representative cube each | 1.299 us |
| 19 item-model buckets, eight representative cubes each | 8.380 us |
| 19 item-model buckets, 32 representative cubes each | 31.949 us |
| four registry-key families, 32 keys each | 1.472 us |
| four registry-key families, 256 keys each | 20.293 us |
| four registry-key families, 1,024 keys each | 67.277 us |

These figures do not claim the full projection cost: item-model baking owns
the inner mesh allocations, and the pre-milestone renderer additionally merges
each layer and creates transient GPU buffers during `render`. The after matrix
must therefore include both retained owner/cache-hit measurements and runtime
apply/residency counters; an outer-vector-only speedup cannot close the
milestone.

## Item / map model-space residency engineering contract (2026-07-19)

This high-risk continuation supersedes phases 2-4 of the broad non-entity
milestone above.  The exact-world-`Arc` cache remains an optional rejection
fast path for projections that are pure functions of one immutable world owner;
it is neither the item residency identity nor completion evidence.  The primary
semantic goal is to replace per-frame world-space item baking, phase-wide mesh
merging, transient vertex/index buffer creation, and repeated map-atlas upload
with persistent resource owners and narrow frame instances, without changing
visible submissions or their ordering.

Completion requires stable item geometry to follow this boundary:

```text
exact resolved model identity + resource/atlas generation
  -> immutable model-space CPU geometry
  -> renderer-owned persistent GPU vertex/index owner
  -> per-frame instance (pose, normal pose, tint/light/overlay, foil, lane/order)
```

An unchanged frame and a partial-tick/camera-only frame upload no item
vertex/index bytes.  A changed transform, light, overlay, foil flag, held-item
pose, bob, rotation, or glint offset uploads only the grow-only instance stream.
A resource reload advances the generation and prevents every old mesh/texture
handle from matching.  One model or map change replaces only that owner; an
ordinary mutation never increments the explicit full-replacement counter.  The
same rules cover dropped, equipped/held, item-frame, first-person, GUI 3D block,
pickup-particle, ominous-spawner, and entity block-attachment consumers across
solid, z-offset-forward, translucent, glint, and glint-translucent lanes.

### Frozen owners and identities

- `bbb-item-model` owns model selection and dynamic property evaluation.  Its
  resolved identity is the exact small ordered model/sprite selection plus tint
  slot topology, matching Vanilla's `TrackingItemStackRenderState`; resolved
  tint values, foil and animation state stay in the narrow instance and it does
  not expose or own a GPU handle.
- `bbb-native` owns visibility and consumer-specific transforms, light,
  overlay, lane and submission order.  It emits model identities, model-space
  build input only on a cache miss, and narrow instances through the single
  `RendererFrame` boundary.  Entity/level ids may identify a transient
  instance but never a resource mesh.
- `bbb-renderer` exclusively owns derived CPU/GPU residency.  An exact owner
  key contains resource generation, atlas generation, atlas/material class,
  render phase where it changes static geometry, and the complete semantic
  model identity.  Hashing may accelerate the table lookup but equality of the
  exact key decides a hit; no vertex/index scan is permitted on the hit path.
- Model-space vertex/index owners are bounded by owner count and logical bytes,
  evict only values not referenced by the current frame, and report the
  explicit measured fallback if a single frame exceeds the bound.  The normal
  path never falls back to per-frame world-space baking.  Level reset drops
  instances but does not poison resource identity; resource/atlas reload drops
  or generations every asset owner.  Entity-id reuse cannot recover an old
  instance because frame instances are cleared and rebuilt.
- The dynamic instance buffer is a render-owned grow-only `FrameDataBuffer`;
  capacity growth is counted.  Draw batching may combine equal consecutive
  owners, but translucent and feature submission order must remain identical.
  A CPU microbenchmark win is rejected if draw calls or pipeline switches cause
  a worse end-to-end frame result.
- Static glint reuses the base model owner.  Standard glint uses static atlas
  UVs and a time uniform; special clock/compass foil derives its sheeted-decal
  UV from model-space position plus the instance pose/scale.  Neither path
  rebuilds geometry for wall-clock animation.

Maps use a separate exact owner contract.  `bbb-world` owns a monotonically
changing per-map content revision (and decoration revision when needed) with
the canonical 128x128 color payload.  The renderer owns one GPU texture per
`(level/session generation, map id, content revision)` lineage, writes only a
changed full image or exact patch, and removes owners on map unload/session or
level reset.  Map backgrounds, decoration sprites and font atlases are
resource-generation owners.  Item-frame and first-person map frames therefore
submit only surface/decorations/text instances.  They never rebuild one
transient atlas containing every visible map.

The world remains the sole canonical mutable owner.  Renderer residency is a
discardable derivation, render/control still pair one exact world `Arc` and
publication generation, and effect-before-publication, blocked-render,
shutdown/join and resource reload barriers remain unchanged.  No protocol,
tick, input, interpolation, visible content, graphics setting, second canonical
store, `unsafe`, allocator hook, Rayon, or configuration UI is introduced.

### Vanilla evidence

Local 26.1 `ItemModelResolver` clears and updates a retained
`ItemStackRenderState`; `TrackingItemStackRenderState` records selected model
objects, foil type and exact tint values as the reusable model identity.
`ModelManager` retains baked item-model maps until its reload `apply` replaces
them.  `ItemFeatureRenderer` submits the same baked quads into solid or
translucent render types, layers standard/special glint from pose and uniform
state, and preserves submit order.  Dropped, held, item-frame, first-person and
ominous-spawner renderers change poses/light around the retained item render
state rather than rebaking the model resource.  `MapTextureManager` owns one
`DynamicTexture` per map id and uploads only while `requiresUpload` is true;
`MapRenderer` rebuilds narrow map/decorative render state against that texture.
`GuiRenderState` clears retained frame containers and also tracks item model
identities for GUI atlas reuse.

### Serial phases, validation and safe stops

1. Extend Criterion with empty and 1/100/500/2,000 instance workloads,
   visibility ratios, unchanged/dynamic/sparse/dense/churn cases, maps and
   reload/retained-generation cases; add logical upload/residency observations
   and record the unoptimized run.  Focused renderer/native/control tests,
   benchmark execution, fmt/diff/docs checks, normal commit.
2. Add the exact resolved identity, model-space CPU/GPU owner and instance
   buffer API.  Tests pin exact-key hit/miss, zero unchanged geometry upload,
   dynamic-only instance upload, reload rejection, bounds/eviction and draw
   order.  Commit and run the milestone full gate; this clean gated tip is the
   first safe stop.
3. Migrate ordinary item consumers and remove their world-space bake/merge/
   transient-buffer path.  Tests compare every visible submission field and
   special phase ordering; commit normally.
4. Add canonical map revisions and persistent per-map texture plus resource
   atlases.  Tests cover one/8/32 patches, unload, level/session reset and
   reload; commit and run the milestone full gate.  This is the second safe
   stop.
5. Migrate remaining item phases, reuse extraction scratch, audit explicit
   special fallback, instance ordering, owner bounds and logical peak memory;
   commit normally.
6. Rerun the identical matrix, record before/after, update architecture,
   unsupported ledger and goal archive, and commit.
7. At the committed final HEAD run the full workspace gate, build
   `bbb-native` into `/tmp/bbb-target-main`, verify the binary timestamp against
   HEAD, then run the explicit Weston-backed live playable gate followed by the
   policy auto gate.

Failures localize to measurement/observation, exact identity/residency,
consumer migration, or map ownership.  Once phases 2 or 4 begin, no safe stop
exists until that owner transition is buildable, reviewed and full-gated.
Failures are fixed forward by normal commits.  A cache uncertainty rebuilds or
uses the explicit bounded fallback without accepting a stale hit; a resource
replacement failure keeps the previous accepted generation until the reload
barrier reports failure.

The phase-1 legacy-path run uses gated base `364426578` plus only the benchmark,
contract and observation changes on the same AMD Ryzen 9 7945HX host and Rust
toolchain recorded above.  Criterion uses ten samples, a 400 ms warm-up and a
two-second measurement window.  The item cases bake six representative quads,
including one translucent face and periodic special foil, into fresh
world-space phase meshes.  The visibility cases deliberately bake all 2,000
candidates before discarding rejected complete owners, matching the cost gap
being removed.  Map cases reproduce id de-duplication, fresh atlas allocation
and full visible-map pixel copies.  They are CPU-owner baselines, not claims
about wgpu submission or complete frame time.

| Legacy item/map workload | Before mean |
| --- | ---: |
| empty / 1 item instance | 6.921 ns / 1.212 us |
| 100 / 500 / 2,000 item instances | 51.161 us / 253.75 us / 2.525 ms |
| 2,000 candidates, 0% / 10% / 50% / 100% retained | 2.526 / 2.551 / 1.834 / 1.827 ms |
| 500 unchanged / camera-only / one transform | 259.14 / 259.80 / 259.17 us |
| 500 one light / one foil / dense animation | 259.31 / 257.75 / 263.13 us |
| 500 item-frame / first-person poses | 266.40 / 260.59 us |
| spawn/remove 500↔250 / retained old frame next generation | 197.76 / 245.70 us |
| transient map atlas unchanged, 1 / 8 / 32 maps | 0.826 / 7.824 / 33.698 us |
| one map patch with 1 / 8 / 32 resident maps | 0.825 / 7.584 / 32.192 us |
| unload one of 32 / resource reload clone+atlas 32 | 38.653 us / 1.193 ms |
| 100 items repeated for 60 / 144 / 260 / 520 frames | 3.168 / 7.580 / 13.606 / 27.584 ms |

The visibility values vary with allocator/drop behavior after truncation, but
all four retain the same O(2,000) bake and demonstrate that rejection does not
avoid model work.  The steady batches scale linearly with render cadence.  The
after run must use the identical cases and pair them with exact logical
vertex/index/instance/map upload counts, resident/peak bytes, owner hit/miss,
draw calls and pipeline switches.

Phase 2 implements this lower boundary in
`bbb-renderer::item_models::residency`.  Its `BTreeMap` key compares resource
generation, atlas-layout generation, atlas/material class, solid/translucent
geometry class, exact model id, exact canonical variant and part id; no digest
or mesh payload decides equality.  `ensure_item_model_resident` accepts a
builder closure and never evaluates it for a hit or stale key.  The renderer
retains immutable CPU vertex/index arrays together with persistent wgpu
buffers, bounded at 4,096 owners and 256 MiB logical payload by default; owners
pinned by the current frame cannot be evicted and overflow is an explicit
counted fallback.  Frame pose, normal pose, tint, light, overlay, foil kind/
scale, phase and submit order are packed into one grow-only instance buffer.

The generated-item input now carries an immutable sprite asset id and reuses a
sprite alpha mask built once when the item atlas is loaded.  `SpriteAlphaMask`
clones share an immutable bit-packed `Arc<Vec<bool>>`, so model selection/
property evaluation no longer rescans atlas pixels or allocates a silhouette
before the residency lookup.
Terrain- and item-atlas replacement advance only their matching atlas
generation; a successful resource-reload barrier additionally advances the
global item resource generation.  Tests pin builder suppression on exact hits,
zero static upload for dynamic-only changes, submit-order retention, stale-key
rejection, atlas-specific invalidation, bounded current-frame pinning and
immutable alpha-mask sharing.

Phase 3 connects that owner to production draws.  `RendererFrame` now carries
one atomic `ItemModelResidencyFrame`: small exact asset requests plus ordered
instances.  The renderer mints generation keys and de-duplicates requests
before evaluating `ItemModelAssetSource`; generated-sprite extrusion and the
model-space-to-GPU bake therefore run only on a miss.  Block-item and
item-frame-border quads are immutable `Arc` assets cached by the loaded terrain
resource owner, so repeated selection does not rebake or copy their quad
payload either.  Dropped items, pickup-particle clusters, ominous spawners,
third-person held/equipped items, first-person hand items, item-frame borders
and contents, and entity block attachments now submit resident instances.
Their ordinary production path no longer appends world-space phase meshes.
At the phase-3 checkpoint, special clock/compass foil remained the explicit
legacy fallback. Phase 5 removed that fallback by retaining model-space decal
coordinates and applying the pose-dependent sheeted-decal scale in the glint
instance/shader path.

The resident pipelines consume persistent model-space vertex/index buffers and
one grow-only instance buffer.  Equal consecutive owner/target/phase records
collapse to one instanced draw without sorting; phase filters preserve the
existing world, item-entity and depth-cleared first-person pass positions.
Static and instance uploads are accumulated until the next actual render and
then consumed, so a second render of the same extraction reports zero upload
rather than repeating stale telemetry. Exact non-empty GPU instance bytes are
also retained across extraction boundaries: an identical payload reuses the
existing grow-only buffer while draw grouping, order, target, phase and scissor
metadata are refreshed. Test builds reconstruct legacy meshes only under
`cfg(test)` to keep the large vanilla pose-equivalence corpus checking final
geometry; that compatibility projection is absent from normal and benchmark
builds.

The phase-3 isolated instance-contract run on the same Ryzen 9 7945HX host uses
the phase-1 Criterion configuration and the same six-quad mixed
solid/translucent model.  It constructs two narrow records per visible item and
retains one shared model-space `Arc`; it excludes wgpu queue time and is paired
with the offscreen upload assertions rather than presented as an integrated
frame timer.

| Item instance extraction workload | Legacy world-space | Resident instance | Speedup |
| --- | ---: | ---: | ---: |
| 500 unchanged | 259.14 us | 18.046 us | 14.36x |
| 500 camera-only | 259.80 us | 18.049 us | 14.39x |
| 500 one transform | 259.17 us | 18.912 us | 13.70x |
| 500 one light | 259.31 us | 18.928 us | 13.70x |
| 500 one foil | 257.75 us | 18.022 us | 14.30x |
| 500 dense animation | 263.13 us | 18.318 us | 14.36x |
| 500 item-frame / first-person poses | 266.40 / 260.59 us | 18.123 / 18.120 us | 14.70x / 14.38x |
| 2,000 candidates, 0% / 10% / 50% / 100% retained | 2.526 / 2.551 / 1.834 / 1.827 ms | 0.396 / 7.696 / 39.034 / 73.732 us | isolated 100%: 24.78x |

The 0-50% resident figures measure the already-frozen visibility-first
contract (reject before full instance construction); they do not by themselves
prove every item producer applies equivalent culling. The production hard
assertion is stronger at the residency boundary: after a first offscreen owner
build, an exact repeated extraction reports zero for all three upload classes;
transform/camera-only replacement reports zero vertex/index bytes and only
instance bytes; and another render without replacement remains zero. The
representative two-owner cube retains 1,392 logical CPU geometry bytes plus
driver-owned GPU buffers; driver-deferred release remains outside logical peak
accounting.

Phase 4 of the item-residency slice moves dynamic map pixels below the same
stable-owner boundary. Canonical `MapColors` is an Arc-COW byte owner with an
allocation generation and monotonic content revision; its serde representation
remains the existing byte array. A retained publication shares the allocation,
and the first valid patch copies only when that old generation is retained.
Invalid and zero-area patches do not advance the pixel revision.

The render-thread extraction cache decodes one full RGBA owner per map and
emits a rectangle only for the next consecutive canonical patch. Repeated
item-frame and first-person uses in one frame share the exact request. The
renderer assigns stable slots in a grow-only square atlas, retains immutable
full pixels for recovery/growth, writes zero bytes for an unchanged revision,
and writes only the patch bytes for a consecutive update. Older retained
publication revisions are counted and rejected. Resource reload and client
level generation changes invalidate dynamic slots plus the persistent
background/decoration atlases; explicit removal and bounded LRU reuse release
unused map owners. The font atlas was already resource-owned and remains so.

The same-host isolated revision-decision Criterion run excludes wgpu queue
time and the still-legacy world-space map-surface merge. Offscreen tests pair
it with exact upload accounting: two initial 128x128 owners upload 131,072
bytes, their unchanged frame uploads zero, and a one-pixel patch uploads four
bytes without replacing the unrelated owner or the full atlas.

| Dynamic map owner workload | Legacy transient atlas | Resident decision | Isolated reduction |
| --- | ---: | ---: | ---: |
| unchanged, 1 / 8 / 32 maps | 0.826 / 7.824 / 33.698 us | 2.441 / 19.747 / 109.46 ns | about 338x / 396x / 308x |
| one patch, 1 / 8 / 32 maps | 0.825 / 7.584 / 32.192 us | 18.336 / 53.637 / 229.13 ns | about 45x / 141x / 141x |

Phase 5 completes the item/map boundary. Map base, background, decoration and
label glyph surfaces use one persistent static quad and narrow surface/glyph
instances. Map pixels use fixed 8x8-slot pages rather than a growing texture:
adding a page uploads only the new map owner and never copies an earlier page.
The persistent table is bounded at 4,096 map owners (64 pages, 256 MiB logical
pixels). If one current frame itself exceeds that bound, explicit current-frame
overflow pages preserve every visible map and increment the fallback counter;
they are not silently omitted or mistaken for persistent residency. Driver-
deferred texture release and current-frame overflow pages remain outside the
logical resident-byte peak.

The same phase migrates special foil and GUI 3D block items. Resident vertices
retain the exact sheeted-decal coordinate, while glint time, pose and decal
scale remain instance/uniform values; clock/compass animation therefore never
rebuilds static geometry. Ordinary hotbar, inventory and debug GUI block items
submit the same exact model identity and narrow pose/tint target into the
resident item pipelines. The target preserves `Gui`, `GuiBeforeBlur` and
`GuiAfterBlur` ordering. At the phase-5 boundary, GUI items whose floating/stats
scissor rectangle required CPU triangle clipping retained a measured legacy
fallback. The 2026-07-28 continuation moved those ordinary block quads to the
same per-draw resident GPU scissor used by Vanilla, preserving exact GUI target
and draw-phase ordering without transient item buffers. Map text similarly
reuses the resource-owned font atlas and static quad rather than constructing a
world-space glyph mesh.

The phase-5 same-host Criterion run uses the same configuration and isolates
the last CPU geometry owners. It excludes queue submission time and is paired
with offscreen output and byte-accounting tests.

| Final migrated workload | Legacy world-space | Resident instance | Speedup |
| --- | ---: | ---: | ---: |
| map surface, 1 instance | 30.296 ns | 7.141 ns | 4.24x |
| map surface, 8 instances | 113.16 ns | 13.323 ns | 8.49x |
| map surface, 32 instances | 545.86 ns | 46.091 ns | 11.84x |
| GUI block item, 1 instance | 1.198 us | 0.107 us | 11.20x |
| GUI block item, 9 instances | 4.618 us | 0.490 us | 9.41x |
| GUI block item, 46 instances | 23.476 us | 2.062 us | 11.39x |
| GUI block item, 100 instances | 51.117 us | 4.476 us | 11.42x |

Production hard assertions cover the cost that the isolated timings omit. An
unchanged second item frame uploads zero vertex/index bytes; transform,
partial-tick, camera, light and foil-only changes upload only instances. Two
initial map owners upload 131,072 pixel bytes, their unchanged frame uploads
zero, and a one-pixel patch uploads four bytes without replacing the unrelated
owner. Ordinary item/map mutations keep both full-residency-replacement
counters at zero. Reload advances the exact resource/atlas generation, level
reset invalidates map owners, entity-id reuse cannot retain an instance, and
the old publication remains immutable.

Runtime observations cross the existing non-waking backend telemetry path and
are published with the next immutable generation: exact item hit/miss/build/
stale counts; recent/peak lookup, extraction and GPU-apply time; created/reused/
replaced owners; vertex/index/instance/map bytes; transient buffers and full
replacement counts; current/peak logical resident bytes; map created/updated/
reused/stale/fallback counts; map surface/instance bytes; and extraction scratch
capacity growth. The exact-world-`Arc` projection cache may reject upper-layer
work, but disabling it still leaves this residency boundary and all zero-static-
upload assertions intact.

Remaining measured boundaries are dynamic/special model selection and property
evaluation before an identity is known, changed-instance uploads, draw/pipeline
state churn, and driver-owned deferred-release memory. Item model residency and
persistent map pixels each allow 256 MiB logical ownership, while a frame that
pins more owners than the bound can temporarily retain explicit
fallback/overflow storage. These are not hidden inside the reported logical
peak. A strict equal-load official Java 26.1 CPU/GPU capture was not produced
by this microbenchmark slice and remains subject to the comparison protocol
below; no isolated section number is used to claim overall client superiority.

Every phase is a normal reviewable commit with affected-crate focused tests,
`cargo fmt --all --check`, `git diff --check`, documentation lint when needed,
and full-diff review. The safe stop after any phase is its clean, full-gated
commit. A cache uncertainty fails open to a rebuild; a renderer replacement
failure keeps the previous accepted owner and surfaces an error rather than
publishing mixed state. If a milestone full gate fails, work stops at that
phase boundary and the failure is fixed forward in a normal commit.

## Stable HUD projection revision-cache contract (2026-07-27)

This bounded engineering continuation removes repeated scoreboard-sidebar and
player-list rich-text, sorting, score-formatting, and profile-skin projection
across immutable publications that changed unrelated world state. Completion
requires the renderer to retain each exact static projection until one of its
semantic inputs changes. A cache hit must skip the projection closure and the
renderer replacement; it is not established by a hash or an approximate
counter.

The world owns two runtime-only monotonic display revisions. The scoreboard
revision advances for applied scoreboard mutations and for player-info or
entity-membership changes that can change the local player's team-specific
sidebar. The player-list revision additionally advances for tab-list changes.
Both revisions survive publication cloning, reset behind the client-level
generation boundary, and are omitted from serialized canonical snapshots.
Invalid packets and ignored mutations do not advance them.

The item runtime separately owns a monotonic profile-presentation revision.
Completed profile resolution, dynamic skin success/failure, and dynamic
texture-session reset advance it before render extraction. Resource generation
and loaded/unloaded state remain separate exact cache-key fields. This prevents
an asynchronously resolved head from being hidden behind an unchanged world
revision.

Native keys the sidebar by client-level generation, scoreboard display
revision, resource generation/load state, and hide-GUI state. It keys the
player-list payload by client-level generation, player-list display revision,
profile-presentation revision, resource generation/load state, visibility, and
the encrypted-session `showHeads` decision. `PlayerTabOverlay` heart
blink/display catch-up remains renderer-owned and receives the current GUI tick
even when the static payload is unchanged. Surface layout, obfuscated glyph
animation, hover/input state outside these projections, and every partial-tick,
camera, or wall-clock value remain frame-local.

Vanilla 26.1 fixes the semantic inputs and ordering:
`GameRenderer.extractGui` resets `GuiRenderState` before
`Gui.extractRenderState`; `Gui.extractScoreboardSidebar` derives the selected
objective and formatted entries from the scoreboard; and
`PlayerTabOverlay.extractRenderState` derives its ordered entries/header/footer
while retaining `healthStates` across visible frames and updating them with
`gui.getGuiTicks()`. Native may retain equal derived payloads, but must preserve
those visibility edges, heart updates, and draw order.

The serial phases are:

1. add world and item-runtime revisions plus focused mutation, clone/reset, and
   async completion tests;
2. add renderer unchanged-payload tick synchronization and native exact cache
   keys, with focused closure-hit, visibility/resource/profile invalidation,
   and heart-state tests;
3. update the gap ledger/archive, run affected-crate tests and the per-commit
   checks, then run the full workspace gate at the engineering checkpoint.

Each phase is a clean, buildable commit. A failed world/item revision test
localizes before renderer integration; a failed native cache test localizes
before the full gate. The safe stop is the last clean phase commit, but the
ledger item remains open until all three phases and the checkpoint gate pass.

## Stable block-entity and debug extraction contract (2026-07-28)

This bounded high-risk engineering continuation removes the remaining
unconditional heap rebuilds for the block-entity and world-debug projections
whose complete semantic inputs can be revision-keyed. Work starts from the
clean, fully gated `756dabdd1`.

Completion requires all of the following:

- Bed, sign board/text, and lectern-book projection is rebuilt only when the
  exact client-level, terrain/light/block-state, block-entity-data, resource,
  or text-filter input changes. Their base instances may be retained, but
  block-destruction crumbling is applied from the current frame after reuse.
- GameTest highlight boxes and labels are rebuilt only when the canonical
  marker set changes. Renderer-owned static highlight values remain separate
  from camera/entity/time-dependent debug values while preserving their final
  Vanilla insertion order.
- Dynamic chest, bell, shulker-box, decorated-pot, banner, enchanting-table,
  conduit, end-portal, skull, and spawner projections remain frame-local
  because they consume partial tick, camera, client tick, animation, profile,
  or entity state. Remote subscription gizmos, water/height/light/solid-face
  debug, selection outlines, and the existing interval-based collision,
  supporting-block, and sky-light caches likewise keep their current dynamic
  owners.
- The four dynamic debug submission vectors are returned from the renderer,
  cleared, and reused by render extraction. Capacity-growth telemetry advances
  only when a reused vector must exceed its retained capacity; invalid input
  sanitization and exact draw ordering remain unchanged.
- Level replacement and resource reload cannot select an old derived value,
  unrelated HUD/entity/inventory mutations do not invalidate either stable
  owner, and an unchanged projection does not call its renderer replacement.

This work changes no protocol, canonical serialized snapshot shape, block
entity visibility, renderer phase order, crumbling stage, text filtering,
debug expiry, camera culling, animation, or resource fallback. It introduces no
wall clock in `bbb-world`, shared mutable renderer state, second world owner,
hash-only cache hit, `unsafe`, background worker, or configuration UI.

Ownership is frozen as follows:

- `bbb-world` owns runtime-only monotonic block-entity-data and GameTest-marker
  revisions. The stable block-entity key combines the existing terrain
  revision with the data revision and client-level generation; ignored or
  invalid updates do not advance it. GameTest replacement, strict expiry, and
  non-empty clear advance only the marker revision. Publication cloning keeps
  the exact revision values, while snapshot restore starts behind a fresh
  level-generation boundary.
- `bbb-native::runtime::render_extract` owns the exact keys, retained static
  bed/sign/lectern base instances, cache hit/miss/rebuild observations, and
  cleared dynamic debug-vector allocations. Resource reload explicitly
  invalidates the block-entity cache; text filtering is part of its exact key.
- `bbb-renderer` owns the last accepted sign-text and GameTest highlight
  values, plus the current dynamic debug values. A frame-level `Unchanged`
  update leaves a stable owner intact. Filled boxes and text are concatenated
  in the existing dynamic-then-GameTest order before one mesh build, retaining
  the shared translucent sorting and always-on-top split.

Vanilla 26.1 fixes the boundary. `LevelRenderer.extractLevel` extracts visible
block entities before block breaking and emits `DebugRenderer` then
`GameTestBlockHighlightRenderer`; `LevelRenderState.reset` clears and retains
its entity/block-entity/debug-facing lists after frame execution.
`BlockEntityRenderDispatcher.tryExtractRenderState` performs validity,
distance, and renderer checks every extraction. `BedRenderer`,
`AbstractSignRenderer`, and `LecternRenderer` derive their retained fields from
block/entity data, light, resources, and text filtering; the other supported
renderers consume dynamic animation/camera inputs and therefore stay
uncached. `GameTestBlockHighlightRenderer` derives its immutable geometry and
label from the active marker map; marker expiry remains a client-tick world
mutation.

Serial stages and failure-localizing checks are:

1. Add the world revision surfaces and mutation/ignored-update/clone/reset
   tests. Focused `bbb-world` tests, format, diff, and docs checks precede the
   normal commit.
2. Add the native exact caches, renderer stable lanes, debug allocation
   recycling, equality/order/invalidation/capacity tests, and source guards.
   Run focused `bbb-native` and `bbb-renderer` tests plus the per-commit checks
   before the normal commit.
3. Delete the closed ledger clause, append the engineering result to the goal
   archive, review the integrated diff, commit, and run the full workspace
   gate at that committed checkpoint.

There is no safe stop inside stage 2's native/renderer owner handoff. A failure
stops at the preceding clean stage commit and is fixed forward. The final safe
stop is the clean stage-3 commit after its full gate; the next slice does not
start earlier.

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

The phase-6 host audit found `client-26.1.jar`, `26.1.json`, 107 downloaded
library artifacts (including the Linux native jars), and `/usr/bin/perf`.
It did not find the version's 451 MB asset index/object tree, launcher/account
profile, a controller capable of replaying the same camera path into the
official client under the headless compositor, or an equivalent external
per-frame CPU/GPU capture tool. `perf` process samples do not supply CPU-frame
p50/p95/p99 or GPU-frame timing, and vanilla internal profiler sections are not
comparable to bbb's internal observations. Therefore no official number is
recorded from this host and the equal-load win criterion is explicitly
unproven. Completing it requires a provisioned official launch plus
deterministic camera replay and one common frame/GPU capture path; downloading
assets alone would not close that measurement contract.

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
layer. A monotonic native resident revision changes on every rebuilt section,
including resource reload, while a level-generation transition clears the old
CPU cache without reusing its GPU identity.

`bbb-renderer` retains the exact vertex/index buffers of every unchanged
section, including siblings of a rebuilt section in the same chunk. One dirty
section removes and recreates only its non-empty layer owners; load/unload is a
desired-owner set difference. The legacy full replacement API remains for
explicit callers and increments a dedicated counter, but the native terrain
path no longer calls it. Camera-dependent translucent sorting retains vertices
and writes only each changed section's index buffer. Renderer telemetry now
distinguishes changed sections, uploaded/removed section-layer owners,
vertex/index bytes, translucent resort owners/index bytes, explicit full
replacements, and current/logical-peak resident bytes.

The offscreen GPU regressions cover both separate chunks and two sections in one
chunk. An unchanged sync reports zero changed/uploaded/removed owners and zero
upload bytes; changing one section reports exactly one changed and one
uploaded/removed non-empty owner while its sibling stays resident. Unloading
then removes only absent owners. Reusing the same numeric revision in a new
level generation still replaces the owner. The section partition test proves
that summed vertices, indices, faces, and source-section count match the former
column mesh.

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
The later section-dirty result below supersedes this historical whole-chunk
invalidation measurement without changing the section/layer GPU ownership.

## Phase 5 Boundary Result

The measured synchronous residual justified one bounded terrain mesh worker:
one representative changed chunk costs about 1.85 ms, which is 27% of a
144-FPS CPU frame budget, while a 32-chunk burst contains about 71 ms of mesh
work. The native render owner now converts canonical world data into immutable
`Arc<TerrainChunkSnapshot>` values and lazily starts one named
`bbb-terrain-mesh` thread. A job contains only the level/mesh/content key,
exact dirty section id/revision rows, target and neighbor snapshot Arcs, an
immutable atlas Arc, camera position, and priority. It contains no `WorldStore`,
renderer, wgpu object, or mutable canonical owner.

The priority queue holds at most four jobs, first prefers chunks with an active
canonical local block prediction, then the nearest selected chunk (the
player/camera chunk has distance zero). A newer queued revision for
the same chunk replaces the old entry; a nearer job may evict the farthest
queued entry. The result channel holds at most five meshes—the queue plus the
single in-flight job—so a blocked render thread cannot create unbounded mesh
memory. Every accepted result must still match client-level generation,
resource/mesh generation, chunk mesh revision, desired selection, and retained
snapshot owner. Accepted partial results replace only their named cached
section layers. Reload/reset cancels queued values, and in-flight stale values
are counted and discarded. GPU creation/upload remains render-thread-only.

Renderer telemetry publishes cumulative queued/completed/stale jobs and
sections, queue-full, current/peak queue depth, recent/peak build nanoseconds,
and recent/peak build section count with the next immutable generation. The
worker propagates a panic on the next render interaction, stops through its
condition variable, and is always joined by the terrain upload owner's drop
path. Focused tests cover bounded priority and same-owner replacement,
immutable source ownership, partial build/result merge, join, stale-result
rejection, reset identity, and counter projection.

## Phase 6 Delivery Boundary

Repository-native implementation milestones include phase 5 and the completed
section-dirty follow-up below. Remaining performance-parity items stay in
`docs/unsupported-features.md` rather than being marked complete without the
strict capture above. Delivery still runs the real bbb/vanilla-server session:
the live gate emits separate
`live smoke passed` and `live vanilla 26.1 session passed` markers only after
the shared evidence assertions. Script contract tests also keep
`input.focus {"focused": true}` before the first synthetic look/key/mouse
operation, because headless Weston exposes no `wl_seat` focus event.

The final validation sequence is deliberately source-bound: run the workspace
full gate, build `bbb-native` into `/tmp/bbb-target-main` from the committed
delivery HEAD, compare binary/commit timestamps, verify both a live Weston
process and its socket, then run `scripts/playable-gate.sh live`. `auto` may run
afterwards as the project-policy smoke, but its fixture fallback is never live
evidence. The artifact directory and exact exit status belong in the delivery
report instead of this stable design record.

## Section-Dirty Terrain Engineering Result

This high-risk engineering slice refined the terrain CPU dirty owner from a
whole chunk to a render section. One block or standalone-light change now
rebuilds and replaces only the Vanilla-invalidated section owners while
unchanged CPU meshes and exact GPU buffers survive. Chunk load/unload, biome
replacement, level reset, and resource reload remain intentionally
conservative full-height boundaries.

The completed boundary includes a typed world-owned section revision surface,
partial immutable worker jobs and results, native merging that retains
unchanged section meshes, renderer revisions at section rather than chunk
granularity, logical queued/built/stale section observations, and same-host
before/after Criterion data. Interior block, vertical section boundary,
horizontal chunk edge/corner, standalone light, full chunk/biome load, unload,
reload, 1/8/32 dirty bursts, retained immutable jobs, and stale results all
receive benchmark or semantic coverage. Snapshot materialization remains
measured separately so the mesh win cannot hide a cost moved back to render
extraction.

This follow-up does not change protocol handling, canonical block/light/biome
values, visible geometry, view distance, the single simulation owner, worker
count or queue bound, GPU-thread ownership, publication generations, or
resource-reload semantics. It adds no second mutable terrain world, `unsafe`,
global allocator, Rayon, or official-comparison claim. Full-height invalidation
is retained wherever a narrower dependency cannot be proved from Vanilla.

The frozen ownership contract is:

- `bbb-world` owns one small revision record per chunk, with a full-section
  baseline plus section revisions. Publication clones retain the record owner;
  a mutation copies only the affected small record, never chunk palette/light
  payloads or a dense world-wide section map. The existing aggregate chunk
  revision remains a derived scheduler/stale guard.
- `bbb-native` owns the cached full immutable cell snapshot and the merge of a
  partial worker result into cached section layers. A job contains exact dirty
  section ids/revisions plus immutable target/neighborhood/atlas Arcs. Results
  are accepted only while level, resource/mesh, aggregate world revision,
  selection, and snapshot owner inputs still match.
- `bbb-renderer` owns a revision per chunk/section resident owner. A changed
  section removes/recreates only its three possible layer owners; an empty
  replacement removes that section without touching siblings. Wgpu values
  remain render-thread-only.

Vanilla 26.1 fixes the invalidation rules. `LevelRenderer.setBlockDirty`
converts the block's `x/y/z ± 1` cube to section coordinates, so only a block
on a section boundary reaches a neighbor section. Standalone light packet
sections call `ClientLevel.setSectionDirtyWithNeighbors`, a 3x3x3 section
range. `handleLevelChunkWithLight`/`enableChunkLight` and biome replacement
dirty the complete level-height 3x3 chunk range. `RenderSection` owns one
stable dirty/compiled section and a rebuild replaces that section's buffers;
translucency resort remains index-only.

Serial stages and safe stops are:

1. Add this contract and the repo-native whole-chunk before workload matrix;
   run renderer benchmark compilation/execution, focused renderer tests, fmt,
   diff/docs checks, commit, and full gate. This is measurement-only and is the
   first safe stop.
2. Add the world section/context revision owner and exhaustive block/light/
   load/unload/biome/reset tests while preserving the aggregate API for the
   still-whole-chunk consumer. Run focused world tests, commit, and full gate.
3. Add the partial section builder, worker payload/result, native cache merge,
   exact renderer section revision, counters, retained-job/stale/reset tests,
   and source guards. Run focused renderer/native/control tests and benchmark,
   commit, and full gate. There is no safe stop inside this owner migration.
4. Rerun the identical matrix, record before/after and remaining snapshot/
   peak-memory risks, update architecture/ledger/archive, commit, and run the
   final full gate. Build `bbb-native` from that committed HEAD in
   `/tmp/bbb-target-main`, verify the binary timestamp, then run explicit
   Weston-backed `playable-gate.sh live` followed by policy `auto`.

Each failure therefore localizes to measurement, world invalidation, or the
worker/renderer owner handoff. Fixes are normal forward commits. The legacy
aggregate consumer makes stage 2 independently buildable; stage 3 is delivered
only after its full gate. A failed live run records its artifact directory,
classifies environment/binary/client/control/timeout/gameplay causes, rebuilds
the current HEAD, and reruns the complete live gate.

The stage-1 before run uses HEAD `8dcb6de05` plus only this measurement harness
on the same AMD Ryzen 9 7945HX host and the Criterion configuration above. Its
64-block-high fixture has four populated sections; the current implementation
still rebuilds every section of every invalidated chunk:

| Dirty workload | Whole-chunk before |
| --- | ---: |
| unchanged retained mesh | 0.739 ns logical reference |
| one interior block | 8.055 ms |
| vertical section boundary | 8.023 ms |
| horizontal chunk edge | 16.118 ms |
| horizontal chunk corner | 33.771 ms |
| one standalone light section | 77.978 ms |
| full chunk or biome load, 3x3 neighbors | 77.892 ms |
| chunk unload, eight neighbors | 68.397 ms |
| 8 interior-block burst | 68.314 ms |
| 32 interior-block burst | 285.07 ms |
| resource reload, 32 resident chunks | 253.53 ms |

The unchanged value is intentionally only the retained-result reference and
not an end-to-end frame claim. Snapshot materialization remains 138.40 us for
one representative chunk, so after data reports it separately from partial
mesh work rather than folding it into an apparent section-mesh speedup.

Stages landed as `db6229ac2` (contract/baseline), `67ed517bc` (world section
revisions), and `cce7b4338` (partial worker/cache/GPU residency). The identical
AMD Ryzen 9 7945HX Criterion matrix after `cce7b4338` reports:

| Dirty workload | Section-dirty after | Before / after |
| --- | ---: | ---: |
| unchanged retained mesh | 0.558 ns logical reference | 1.33x |
| one interior block | 1.641 ms | 4.91x |
| vertical section boundary | 3.278 ms | 2.45x |
| horizontal chunk edge | 3.208 ms | 5.02x |
| horizontal chunk corner | 7.866 ms | 4.29x |
| one standalone light section | 55.605 ms | 1.40x |
| full chunk or biome load, 3x3 neighbors | 76.259 ms | 1.02x |
| chunk unload, eight neighbors | 67.453 ms | 1.01x |
| 8 interior-block burst | 12.868 ms | 5.31x |
| 32 interior-block burst | 61.264 ms | 4.65x |
| resource reload, 32 resident chunks | 254.87 ms | 0.99x |

The resource-reload value is the isolated rerun after the first all-cases pass
ended thermally at 284.05 ms; 254.87 ms is within 0.5% of its 253.53 ms
full-height baseline. Full chunk/biome and unload deliberately remain
full-height and are effectively unchanged. One-chunk snapshot materialization
is 138.72 us after versus 138.40 us before. The 8/32 snapshot cases measured
9.261/40.972 ms, about 2% above that earlier run; this code path did not change
and remains a separate extraction hotspot rather than hidden mesh work.

World publication exposes the cost of the new retained revision owners:
unchanged 625-column publication moved from 10.650 to 13.565 us, and single
block mutation plus publication from 20.530 to 24.063 us. This is an explicit
3.5 us absolute publication cost, not work deferred to the next mutation. A
representative sum of the separately measured single-block publication,
one-chunk snapshot materialization, and interior mesh phases falls from about
8.214 to 1.803 ms, or 4.56x. It is an owner-phase sum, not a claimed integrated
frame timer. A future revision-store layout must beat both publication and
mutation totals; wrapping the whole map in COW would merely move a 625-entry
copy to the first retained-generation mutation and is rejected.

Old publications retain only the prior small changed revision records while
chunk palette/light and unchanged records stay shared. The CPU cache retains
one full immutable cell snapshot and merged section meshes per selected chunk;
the bounded worker can additionally retain one in-flight plus four queued
snapshot/atlas jobs and five completed partial results. Local updates reduce
result mesh payload, but driver-deferred release of replaced GPU buffers remains
outside the logical resident-byte peak. Equal-load official Java frame capture
is still blocked by the already recorded missing official asset/account/camera
replay and common CPU/GPU capture setup, so these repo-native before/after
numbers are not presented as an official-client win.

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
