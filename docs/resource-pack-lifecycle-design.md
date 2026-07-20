# Resource-Pack Stack and Reload Lifecycle

This document freezes the high-risk engineering contract for the P3 resource
slice. It refines the ownership rules in
[`native-client-architecture.md`](native-client-architecture.md) and the
effect/publication barrier in
[`simulation-render-thread-design.md`](simulation-render-thread-design.md).
The goal is one selected client-resource stack and one reload generation across
startup packs, server packs, CPU assets, GPU/audio derivations, and downloaded
textures. It is not a new configuration UI or a second canonical world.

## Semantic Goal and Completion

The completed path is:

```text
validated selected pack identities (low -> high priority)
  -> immutable resource generation and exact source diagnostics
  -> prepare every CPU asset against that generation
  -> stage every renderer/audio derived owner
  -> one commit barrier
  -> simulation acknowledgement and server APPLIED response
```

Completion requires all of the following:

- vanilla, startup-command-line, and server packs have an exact id, source,
  root, format/compatibility, selection policy, and deterministic priority;
- the mandatory vanilla pack is bottom priority, selected startup packs retain
  command-line order, and active server packs use Vanilla's downloaded-pack
  ordering rather than whichever order paths happened to be appended;
- metadata, overlay and filter parsing is performed once by the pack owner and
  malformed packs fail before a consumer reads their payload; compatibility is
  recorded exactly, while an explicitly selected incompatible pack remains
  selected as it does after Vanilla's confirmation boundary;
- one generation prepares language/font, atlases/textures, blockstate/model,
  item model, particles, waypoint styles, sounds, and world defaults, then
  stages GPU/audio owners without mutating the accepted generation;
- commit either makes all prepared owners visible or none of them visible;
  recovery never leaves a mixed old/new atlas, model, font, sound, residency,
  or simulation projection;
- server `SUCCESSFULLY_LOADED` follows the successful commit and a failed
  activation keeps the prior accepted stack while rejecting stale asynchronous
  results by generation and pack identity;
- downloaded resource textures share one bounded pending/ready/failed cache
  contract, and profile/skin/cape/elytra plus non-profile consumers cannot draw
  a stale handle from a previous resource or session generation;
- control diagnostics identify selected pack, source, priority, generation,
  preparation/commit failure stage, cache state, and stale-result count without
  becoming canonical state; and
- two-pack fixtures and the explicit live playable gate prove push/apply/pop,
  recovery, presentation change, shutdown, and current-HEAD binary provenance.

## Non-Goals

- No protocol, tick, input, visibility, animation, render ordering, or server
  authority change.
- No in-game pack/options UI. Startup selection remains CLI-supplied and server
  prompting follows the configured startup policy/control surface.
- No independently mutable asset world inside `bbb-world`, renderer, audio, or
  control. World keeps only canonical server-pack presentation/state.
- No stale-on-error behavior, silent content omission, global allocator,
  `unsafe`, Rayon, or unbounded download/decode/upload queues.
- No claim that arbitrary mod loaders or a changed vanilla registry catalog are
  supported. Custom resource-pack namespaces and synchronized datapack assets
  still follow the Java 26.1 client/resource and RegistryAccess boundaries.

## Current Baseline and Cost/Risk Ownership

Baseline commit `0079b95e5` uses the warm main target and serial focused tests:

| Existing boundary | Baseline evidence |
| --- | ---: |
| resource lookup/filter/overlay tests | 12 passed |
| server ZIP/metadata safety tests | 4 passed |
| pack-backed consumer precedence tests | 10 passed |
| server download/activation/recovery tests | 13 passed |
| native reload-barrier tests | 4 passed |

The existing implementation already bounds a server download to 250 MiB,
validates ZIP paths/counts/sizes/ratio and pack format, keys asynchronous work
by UUID plus push sequence, and delays `SUCCESSFULLY_LOADED` until a renderer
reload acknowledgement. It also prepares audio, particle, item/language/font,
block-destroy and block-sound CPU state before applying renderer resources.

The remaining ownership risk is structural. `PackRoots` stores a bare ordered
`Vec<PathBuf>` with no selected-pack identity or compatibility record. Startup
directories are not admitted by the same metadata owner as downloaded packs.
Native then mutates particle/item/terrain/HUD/entity/sky renderer owners in
sequence and attempts to reconstruct the previous owners if a later upload
fails. A recovery upload can itself fail, so that procedure is not an atomic
generation. Dynamic profile and texture runtimes also own independent cache
state and invalidation rules.

The bare path order cannot explain Vanilla's simultaneous server-pack behavior
by itself. `DownloadedPackSource.loadRequestedPacks` reverses the
`ServerPackManager` push-ordered list before assigning repository ids; the
repository then discovers those ids in sorted order and inserts each required
fixed-top pack before existing fixed-top packs. Those two reversals restore
push order in the final low-to-high selected list, so the latest pushed pack has
highest resource priority. The selected-stack milestone must lock the final
behavior with two conflicting resources, not merely assert an intermediate
path list.

## Frozen Ownership and Integration Contract

- `bbb-pack` owns `SelectedClientPack`/selected-stack value types, metadata and
  format validation, overlay/filter selection, source-aware resource lookup,
  immutable generation inputs, and minimal two-pack fixtures. A resource result
  carries or can report its exact selected pack identity.
- `bbb-world` remains the only canonical server-push/prompt/response owner. It
  stores no path, decoded texture, model, audio catalog, task, or backend handle.
- `bbb-native::resource_pack_runtime` owns connection-scoped download/materialize
  tasks and maps UUID+push sequence to selected server packs. It never marks a
  pack active before the matching reload generation commits.
- `bbb-native::resource_reload` owns the transaction and dependency order. Its
  prepared generation is immutable and `Send`; it contains simulation
  projections plus discardable renderer/audio staging values, not live owner
  borrows. Only a matching generation may commit or acknowledge.
- `bbb-renderer` and `bbb-audio` exclusively own their derived handles. Each
  exposes prepare/stage and commit/drop boundaries; a staging failure drops new
  values without changing accepted handles. Item/map/terrain generations and
  background workers use the committed resource generation in exact keys.
- `bbb-control` exposes serde-defaulted observations only. It never selects a
  pack, retains an asset payload, wakes simulation per frame, or reads a live
  mutable runtime.
- Downloaded dynamic textures use an exact key containing resource/session
  generation, resource identity and content identity. A digest may accelerate
  lookup but exact identity decides a hit. Queue bounds, cancellation, failure
  and default/missing fallback are explicit; old results are discarded.

Resource priority is represented low-to-high because the existing
`PackResourceStack` and Vanilla `MultiPackResourceManager` both push in that
order and resolve the last surviving resource. Server pack response order is a
separate protocol fact and must not be inferred from resource priority.

## Implemented Selection and Preparation Boundary

The selected-owner milestone now gives every admitted pack an exact source,
selection policy, compatibility range and low-to-high priority. Startup pack
metadata is read before native subsystems start; downloaded-pack
materialization retains the same parsed metadata document that later enters
selection. `PackRoots` keeps an `Arc<ClientResourceGeneration>` containing the
selected stack and one precompiled `PackResourceStack`. Resource queries clone
that owner rather than rereading `pack.mcmeta`, recompiling filter regexes or
recomputing client/data overlay roots per consumer.

Startup uses generation zero. Each server reload request prepares its stack
with the request generation before any audio, item, particle, terrain, HUD,
entity or sky consumer runs. Malformed filters/overlays therefore fail at the
preparation boundary, while an explicitly admitted incompatible format remains
visible in the selected-pack compatibility record. Resource lookup results
carry the exact selected pack id. The renderer/audio apply side is still the
next phase: it currently mutates subsystem owners sequentially and relies on
recovery uploads after failure.

## Vanilla 26.1 Evidence

- `ClientPackSource` creates mandatory `vanilla` at `BOTTOM`; optional built-in
  packs default to `TOP`.
- `PackRepository.rebuildSelected` preserves selected ids and inserts missing
  required packs according to `PackSelectionConfig` and fixed position.
- `Pack.readPackMetadata` validates client-resource metadata, compatibility,
  requested features and overlays before selection.
- `MultiPackResourceManager` consumes selected packs in low-to-high order and
  applies namespace/path filters at the pack boundary.
- `DownloadedPackSource` gives downloaded packs required fixed-top selection,
  reverses the requested server list during materialization, and sends APPLIED
  only from `onReloadSuccess`.
- `ServerPackManager` keys replacement/removal by UUID, batches downloads,
  holds activation at PENDING during reload, preserves prior ACTIVE packs for
  recovery, and maps failure stages to DECLINED, DOWNLOAD_FAILED, DISCARDED or
  ACTIVATION_FAILED.
- `Minecraft.reloadResourcePacks` admits only one pending reload, rebuilds the
  selected repository, drives registered listeners through one reload instance,
  calls `LevelRenderer.allChanged` after success, and otherwise enters the
  rollback/recovery path.

## Serial Phases, Tests, and Checkpoints

1. **Evidence and contract.** Record the baseline above, add this contract and
   freeze exact source/priority/generation terms. Documentation checks plus the
   existing pack/native focused tests produce a normal commit.
2. **Selected pack owner.** Add validated pack identity/metadata/selection
   values, project startup and server sources into one low-to-high stack, fix
   simultaneous server-pack priority, and attach source diagnostics to resource
   results. Focused `bbb-pack` and `bbb-native` tests cover malformed metadata,
   incompatible formats, required/fixed insertion, filters/overlays, conflicts,
   replacement and disconnect. Commit and run a high-risk full gate.
3. **Immutable prepared generation.** Build every CPU consumer from one selected
   stack and generation; remove ad-hoc root composition and independent
   re-reads. Focused pack/item-model/native/audio/world tests cover exact
   generation and unchanged canonical world state. Commit normally.
4. **Atomic renderer/audio staging.** Replace mutate-then-recover with staged
   subsystem owners and one commit; test injected failure after every stage,
   old-handle retention, item/map/terrain invalidation and background stale
   rejection. Commit and run a high-risk full gate.
5. **Unified dynamic textures.** Introduce the bounded generation-keyed cache,
   migrate existing profile/skin/cape/elytra and selected non-profile consumers,
   and preserve exact vanilla fallback. Focused HTTP-free fixtures cover
   pending/ready/failed, deduplication, cancellation, id reuse and session/reload
   reset. Commit normally.
6. **Custom-resource composition and diagnostics.** Close resource overrides and
   the scoped custom model/texture/sound/font consumers exposed by two minimal
   packs; publish observations and audit every remaining consumer/cache. Commit
   and run a high-risk full gate.
7. **Closeout.** Update ledger/archive/architecture, run final focused tests,
   docs/fmt/diff checks and the final full gate. Build `bbb-native` from the
   delivered HEAD, verify its timestamp, then run explicit Weston-backed
   `scripts/playable-gate.sh live` and the policy `auto` gate.

No phase may introduce a selectable old/new resource path. The safe stops are
the clean full-gated tips after phases 2, 4 and 6. Before those checkpoints a
failure is localized to selection, preparation, staging or dynamic cache and is
fixed forward with a normal commit. If an exact identity or generation cannot
be proven, the new value is rejected or rendered through the explicit missing/
default fallback; a stale accepted handle is never used as a convenience.
