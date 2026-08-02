# Resource Image Aggregate Memory Budget

> Canonical scope：资源可信链见
> [`docs/wiki/mechanisms/resources/offline-asset-trust.md`](wiki/mechanisms/resources/offline-asset-trust.md)。
> 本页只保留 image/atlas memory budget 的详细 evidence。

This document freezes the high-risk engineering contract for bounding decoded
resource images across an atomic resource reload and the unified downloaded
profile-texture owner. It extends
[`resource-input-safety-design.md`](resource-input-safety-design.md) without
changing pack selection, visible fallback, or renderer ownership.

## Semantic goal and completion criteria

Pack-backed PNG admission must happen before pixel allocation at three nested
levels: the existing per-image checks, the existing 256 MiB per-atlas charge,
and one generation-wide charge shared by every atlas, standalone resource
texture, colormap, font, entity, HUD, sky, and profile-resource decode. A staged
reload receives only the process allowance left after the accepted generation's
exact logical charge. Failure aborts preparation and leaves the accepted
generation and its renderer/audio handles untouched.

Downloaded skin/cape/elytra work must retain at most 256 MiB of ready decoded
pixels in its unified owner. Admission evicts least-recently-used non-pending
entries before accepting a result. The result handoff is bounded so only one
maximum-size worker result and one maximum-size renderer-upload staging value
sit outside that resident charge. Unknown, stale, oversized, or unadmittable
results use the existing explicit failed/fallback path.

The slice is complete when:

- unchanged generation clones share one exact counter rather than creating an
  independent allowance;
- every production pack PNG decode is charged before allocation, and source
  dimension-only atlas entries cannot bypass the same charge;
- the accepted plus staged logical charge never exceeds the 1 GiB process
  ceiling, while one generation cannot exceed 512 MiB;
- rollback releases the staged generation charge and commit releases the old
  charge only after the renderer transaction succeeds;
- dynamic ready bytes never exceed 256 MiB, admission/eviction/rejection and
  worker/staging bytes are observed without scanning payloads;
- generation/session reset, stale worker results, queue saturation, result
  delivery, upload failure, and entry-id reuse cannot revive an old image;
- source guards reject new pack decoders or dynamic result paths that bypass
  these owners.

## Baseline and risk attribution

The per-image safety audit found 3,665 official Java 26.1 PNG resources, with
largest decoded payloads of 1 MiB and roughly 35.2 MiB of raw RGBA across the
current local `assets/**` inventory. That is content evidence, not the runtime
peak: BBB may decode the same selected resource for multiple derived consumers,
and each atlas currently receives an independent 256 MiB allowance.

Before this slice there is no generation-wide or accepted-plus-staged cap. The
dynamic owner bounds 2,048 entries and 64 queued requests but not bytes. Since a
valid cape/elytra payload may decode to 64 MiB, its theoretical ready-image
ceiling is 128 GiB; a drain may clone every delivery-pending image into a
temporary result vector. These are logical worst cases rather than measured
ordinary gameplay residency, but they are reachable ownership bounds and must
not be reported as allocator measurements.

Repo-native tests will record exact logical bytes at every admission and assert
the limits without wall-clock thresholds or a global allocator. The after
closeout will record the official startup/reload charge observed on this
machine and the remaining renderer/GPU-derived peak separately.

## Ownership and integration contract

- `bbb-pack::image` owns constants, checked arithmetic, per-atlas and
  generation-wide reservations, plus immutable observations. A
  `ClientResourceGeneration` and every `PackResourceStack` derived from it share
  the same generation counter.
- `PackRoots` creates unmanaged/test generations with an isolated allowance and
  selected generations with the explicit staged allowance supplied by native.
  Consumers request a bounded decode through pack APIs; they never mutate or
  reset counters themselves.
- `bbb-native::resource_reload` reads only the accepted generation's logical
  charge, computes the staged allowance before preparation, and keeps the old
  roots alive through the existing renderer transaction. It does not count GPU
  buffers as decoded PNG bytes or free accepted owners early.
- `bbb-item-model::skin_runtime` owns the remote LRU, bounded request/result
  channels, ready bytes, and handoff counters. Exact resource/session/kind/URL/
  handle identity remains the semantic key; a hash never proves equality.
- `bbb-renderer` keeps its existing CPU/GPU dynamic atlas owner. This slice
  bounds the incoming remote cache and one upload value; it does not claim that
  driver-deferred GPU release or atlas rebuild scratch is covered by the CPU
  decoded-image counter.
- `bbb-control` exposes serde-defaulted logical observations only. It never
  retains pixels, changes admission, or wakes simulation per frame.

## Implemented outcome and measured evidence

`bbb-pack::image` now admits every production pack PNG through one shared
`ResourceImageGenerationBudget`. One generation is limited to 512 MiB of
logical RGBA, and native computes a staged reload allowance from the accepted
generation's exact charge so the two adjacent generations cannot exceed 1 GiB.
Cloned stacks share the same counter; a new resource generation receives a new
counter. A failed decode keeps its conservative pre-allocation charge until that
generation is dropped, so malformed input cannot repeatedly reclaim and reuse
the same allowance.

On this machine, the ignored all-official-atlas coverage test charged
12,901,824 bytes (12.30 MiB), or 2.40% of the 512 MiB generation ceiling. The
earlier filesystem inventory estimate was 35.2 MiB for every official PNG;
these values differ because the runtime test measures selected atlas consumers,
not every PNG stored under `assets/**`. They are logical owner measurements,
not allocator RSS.

The unified downloaded profile-texture runtime now admits at most 256 MiB of
ready decoded pixels and evicts the least-recently-used ready owner that is not
awaiting delivery. Its zero-capacity rendezvous result channel retains at most
one worker result outside that owner, and each render pump takes at most one
profile result. With the 64 MiB per-image ceiling, the structural decoded-pixel
bound is therefore approximately 384 MiB: 256 MiB ready residency, one 64 MiB
worker result and one 64 MiB renderer-delivery value, plus at most one small
skin delivery. Before this slice, the 2,048-entry owner alone admitted a
theoretical 128 GiB and the unbounded result/drain path could retain additional
copies.

Runtime diagnostics report the generation limit/current/peak charge and the
dynamic resident limit/current/peak, eviction, rejection, worker-result and
delivery bytes. They do not include compressed input, decoder scratch,
renderer CPU-atlas rebuild scratch, GPU allocations or driver-deferred release.
Those exclusions remain explicit peak-memory risks rather than being inferred
from the logical counters.

## Non-goals

- No protocol, tick, input, visibility, animation, texture priority, resource
  fallback, or render-order change.
- No global allocator instrumentation, OS RSS assertion, `unsafe`, Rayon, or
  extra canonical owner.
- No silent selected-resource omission to remain under budget. A required
  preparation failure rejects the staged generation atomically; existing
  explicitly optional consumers keep their existing warning/fallback policy.
- No JSON/model/font collection or audio-duration budget. Those remain a
  separate unsupported input-safety boundary.
- No incremental dynamic GPU atlas or driver-release redesign unless focused
  evidence shows the byte admission cannot be made correct without it.

## Vanilla 26.1 evidence

- `SpriteResourceLoader.loadSprite` and `TextureContents.load` read selected
  resources through `NativeImage.read` before atlas stitching.
- `AtlasSet.scheduleLoad` prepares atlas data as reload work and applies it only
  at the reload barrier.
- `Minecraft.reloadResourcePacks` retains the accepted resource manager and
  renderer state until the new reload succeeds or recovery restores it.
- `SkinTextureDownloader.downloadSkin` decodes downloaded/cache bytes and only
  applies legacy conversion for skins; `SkinManager.TextureCache` retains the
  per-resource asynchronous result and fallback behavior.

The byte ceilings and eviction policy are BBB defensive limits. Vanilla defines
the lifecycle and visible behavior, not these numeric bounds.

## Serial phases and gates

1. **Contract and baseline.** Check in this owner contract and the structural
   worst-case baseline. Documentation lint is the focused validation.
2. **Pack generation admission.** Add shared generation accounting, migrate all
   production pack PNG consumers, and test cross-atlas aggregation, clone
   sharing, custom low limits, failed decode, and source guards. Run the first
   milestone full gate.
3. **Atomic reload integration.** Compute staged allowance from the retained
   accepted generation before decoding, expose immutable diagnostics, and test
   success, rollback, repeated reload, and no early release.
4. **Dynamic texture admission.** Add the resident-byte LRU, bounded result
   handoff, one-result delivery budget, counters, and reset/stale/upload-failure
   coverage. Run the second milestone full gate.
5. **Closeout.** Record after logical bytes and residual renderer/GPU peak,
   update architecture/unsupported/archive ledgers, run final focused checks and
   full gate, build `bbb-native` from final HEAD, then run explicit live and
   policy auto playable gates.

Every phase uses normal commits. Failures are fixed forward. The only safe
stops before final delivery are clean full-gated milestone tips; a staged
generation or dynamic result that cannot prove admission is rejected rather
than partially published.
