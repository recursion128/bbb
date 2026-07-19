# Remote Debug Subscription Design

This document is the pre-start contract for the high-risk remote debug
subscription engineering slice. The slice completes the Java 26.1
`ClientDebugSubscriber` path without changing ordinary gameplay or making debug
state a second world authority.

## Semantic Goal And Completion

Decode every value-bearing entry in the fixed 16-entry vanilla debug
subscription catalog into typed protocol values, retain requested values in the
canonical client world with vanilla key, removal, expiry, and level/entity/chunk
drop semantics, and project the values into the matching debug gizmo consumers.
The existing dedicated-server tick-time chart remains part of the same request
set and sample path.

Completion requires:

- block, chunk, entity, and event packets dispatch through the generated debug
  subscription catalog and reject malformed or codec-less payloads;
- the serverbound request carries the complete deterministic requested set;
- canonical maps are keyed by block position, chunk position, or entity UUID,
  while events retain their remaining lifetime;
- packets for subscriptions that were not requested and entity packets for an
  absent protocol entity are ignored exactly as vanilla does;
- temporary values expire on client game time, and chunk unload, entity remove,
  level replacement, disconnect, and subscription changes remove the same
  owners as `ClientDebugSubscriber`;
- POI and game-event holder ids resolve through the immutable registry revision
  owned by the same `WorldStore` generation, while the debug-subscription and
  block registries retain their fixed generated-catalog semantics;
- enabled remote debug renderers consume only one immutable publication
  generation and use the existing line, filled-box, and text-billboard renderer
  channels; and
- protocol, world, net, simulation/publication, render extraction, level-reset,
  and old-generation isolation tests pass, followed by the declared full and
  playable gates.

## Non-Goals

- Do not change packet ids, tick cadence, gameplay input, ordinary rendering,
  or server authority.
- Do not implement the separate GameTest highlight packet renderer in this
  slice.
- Do not add an in-game configuration screen or reuse F3 debug-profile entries
  for vanilla `SharedConstants` development flags.
- Do not make raw payloads, protocol entity ids, or session holder ids into a
  parallel canonical schema.
- Do not add a second mutable debug world, a render-owned copy, locks around
  `WorldStore`, `unsafe`, or a new thread owner.
- Do not invent client AI, pathfinding, POI state, raids, structures, or game
  events when the server did not publish them.

## Ownership And Integration Boundaries

- `bbb-protocol` owns generated subscription ids, typed update/event values,
  nested value codecs, bounds validation, and the full request-set encoder.
- `bbb-net` transports the complete subscription-id set and does not interpret
  values.
- `bbb-world` owns the one canonical `ClientDebugSubscriber` analogue: requested
  subscriptions, keyed value maps, event lifetimes, holder resolution, drop
  hooks, counters, and narrow immutable queries.
- `bbb-native` derives the requested set from startup-only development renderer
  flags plus the existing FPS-chart state, sends changes from the simulation
  owner, ticks expiry in vanilla order, and extracts gizmos from one immutable
  world generation.
- `bbb-renderer` keeps only backend-ready debug primitives. It does not retain
  subscription maps or resolve registries/entities.
- `bbb-control` continues to reuse the exact published world `Arc` and
  generation; no control-specific debug copy is added.

The startup surface is one repeatable, default-empty remote-debug-renderer
selection whose named groups map to vanilla development flags and their exact
subscription groups. It is startup configuration, not an in-game UI.

The canonical state implementation uses one generated subscription catalog as
the id, value-codec, and expiry owner. Each requested id owns block-position,
chunk-position, entity-UUID, and event collections; position-keyed collections
serialize as ordered entry sequences so `WorldStore` JSON never depends on
non-string object keys. The generated `GameEvent` field order and `PoiTypes`
bootstrap order resolve nested holder ids to resource identities before the
generation is published. Chunk forget and entity removal delete their keys,
temporary values expire against client game time, listener clear drops the
request set, and dimension-only `dropLevel` rebuilds empty maps for the retained
set.

## Vanilla 26.1 Evidence

- `DebugSubscriptions` registers the fixed built-in catalog in ids `0..=15`.
  Only dedicated-server tick time has no value codec. Temporary expiries are
  entity/block intersection `100`, redstone orientation `200`, neighbor update
  `200`, and game event `60` ticks.
- `DebugSubscription.Update.STREAM_CODEC` writes the subscription registry id
  followed by an optional value; `Event.STREAM_CODEC` writes the id followed by
  a required value. `holderRegistry` fields are raw ids in the current registry
  access.
- `ClientDebugSubscriber.tick` changes the request set before purging temporary
  values. Updates are ignored unless their subscription map exists. Entity
  updates are ignored when the protocol entity is absent and are then keyed by
  UUID.
- `dropChunk` removes one chunk key and every block key inside that chunk;
  `dropEntity` removes its UUID from every active subscription; `dropLevel`
  clears values while retaining/reinitializing the requested set.
- `ClientPacketListener.tick` calls the subscriber with the level game time.
  Its packet handlers apply block/chunk updates at current game time and resolve
  an entity before applying an entity update.
- The matching vanilla consumers are bee/hive/goals, brain/POI, breeze, entity
  paths, entity-block intersections, redstone orientations, village sections,
  raids, structures, game-event listeners/events, and neighbor updates.

Authoritative files are under
`~/Work/mc-code/sources/26.1/net/minecraft/{util/debug,client/multiplayer,client/renderer/debug}`
plus `world/level/pathfinder` and `world/level/redstone/Orientation.java`.

## Delivered Ownership And Presentation

The delivered simulation owner derives one sorted request set from the
repeatable startup `--remote-debug-renderer` selection and the live FPS-chart
toggle. It sends only changes, updates the canonical requested set only after
the bounded network queue accepts the command, retries after backpressure or a
fresh-listener clear, and purges temporary values at the current pre-increment
game time. Disconnect clears request/value ownership without erasing packet
counters or independent debug-overlay diagnostics.

Render extraction consumes the startup selection and typed maps from the same
immutable publication as the rest of the frame. Empty selection returns before
entity picking or transform projection. Block/chunk-only groups also skip the
entity table; entity-backed groups build one transform/UUID/id index and share
it across all selected consumers. Bee flower, hive-member, blacklist, and ghost
hive data are pre-aggregated once rather than rescanning all bees per hive.
Brain selection uses vanilla's render-then-update ordering, remembered target,
eight-block entity ray, spectator gate, and client-level reset. UUID labels use
the exact Java UUID hash and legacy LCG name tables.

All thirteen remote renderer groups lower into the existing immutable filled-
box, line, and always-on-top text-billboard channels. This preserves canonical
value, distance, expiry, selection, and renderer-group ordering without a
render-owned subscriber copy. The generic line channel represents vanilla
arrows as line segments (without `ArrowGizmo` heads), and Breeze attack-target
lines currently sample the published entity transform rather than a separate
render-owned interpolated transform cache. Exact arrow-head geometry or that
sub-frame visual refinement belongs to the generic debug-gizmo renderer, not a
reopened protocol or canonical-state path.

## Serial Phases And Validation

1. **Typed protocol and request set.** Replace raw debug value payloads with the
   generated-catalog-dispatched update/event model, add every nested codec and
   malformed/bounds fixture, and generalize the serverbound encoder/net command
   to the complete deterministic set. Focused tests: `bbb-protocol` debug-game
   and packet-coverage tests plus `bbb-net` command tests. This phase ends in a
   normal buildable commit and a full gate because it changes protocol codecs.
2. **Canonical subscriber lifecycle.** Add requested-set gating, typed keyed
   maps/events, expiry, nested holder resolution, and chunk/entity/level/drop
   hooks. Add serde, publication isolation, reconnect/respawn/unload/remove, and
   packet-apply tests. Focused tests: `bbb-world` debug-game, level, entity, play
   apply, registry, and publication tests. This phase ends in a normal buildable
   commit and a full gate because it changes canonical state ownership.
3. **Simulation request owner and debug presentation.** Add the startup-only
   named renderer groups, compute/send the exact request set in simulation tick
   order, and project the canonical maps into existing generic renderer
   primitives. Add subscription-change, effect/publication pairing,
   blocked-render, render extraction, and startup tests. Focused tests:
   `bbb-native`, `bbb-net`, and affected `bbb-renderer` tests. This phase ends in
   a normal buildable commit and a full gate because it crosses the simulation
   publication boundary.
4. **Coverage and closeout.** Update packet coverage, unsupported-feature and
   goal archives, registry-holder coverage where applicable, and this design
   with delivered behavior and residual limits. Run documentation lint, focused
   tests, formatting, diff checks, complete diff review, a final full gate at
   delivered `HEAD`, then `scripts/playable-gate.sh auto`.

Every phase also runs `cargo fmt --all --check`, `git diff --check`, and the
documentation lint when documentation changed before its commit.

## Failure Localization And Safe Stops

Protocol decode, world retention, and renderer extraction are kept in separate
serial commits so a failing full gate identifies the owning boundary. Failures
are fixed forward with normal commits; history is not amended or rebased.

The safe stopping points are the clean, fully gated phase tips. A real blocker
in a later phase leaves earlier typed/canonical behavior buildable and covered,
but the slice is not declared complete and its remaining contract stays in
`goal.md`/`unsupported-features.md`. No later phase may introduce a fallback raw
decode path or retain independently mutable presentation state.
