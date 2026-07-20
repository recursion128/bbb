# Entity Spawn/Default Foundation Engineering Contract

This document freezes the high-risk engineering slice that closes the
`spawn/default` dimension for the fixed Vanilla 26.1 EntityType catalog. The
starting ledger has one complete row (`minecraft:turtle`) and 156 partial rows.
The previously completed AddEntity lifecycle proves packet admission and
recreation ordering, but it does not by itself prove every factory constructor,
default attribute value, client-observable field default, or reset behavior.

## Semantic Goal and Completion

For every one of the 157 generated EntityType entries, the client must create
the exact Vanilla runtime family and expose the same client-observable fresh
state before later metadata, attributes, equipment, passengers, events, or
ticks arrive. A successful same-id AddEntity must reset the same owners to
fresh values. A rejected factory admission must leave the existing entity
unchanged, and a retained immutable publication must keep the old generation.

The authoritative chain is:

```text
session EntityType holder
  -> exact resource identity
  -> generated fixed EntityType entry
  -> generated runtime-class / inheritance / default-attribute profile
  -> narrow canonical spawn owners
  -> existing packet-specific recreateFromPacket specializations
```

The slice is complete when every `spawn/default` row is `complete` or has a
Vanilla-proven `not-applicable` status; the checked-in generator proves exact
catalog coverage and fails on a new or structurally changed factory/default;
all defaults consumed by world/render/control queries come from the generated
profile or a documented family implementation; same-id re-add, id reuse,
disconnect/reset, registry reorder, and retained-publication isolation have
cross-family tests; and the final full, explicit live, and auto gates pass.

## Non-Goals

- Do not close metadata callbacks, later interpolation/tick/movement,
  dimensions/pick, renderer/model/layer, animation, equipment/passenger/leash,
  event/particle/audio, or save/debug cells merely because an entity spawns.
- Do not port server AI, goals, navigation, spawning, combat results, loot,
  breeding, server physics, or server-private constructor state.
- Do not invent a second canonical entity world, a native-owned entity cache,
  a parallel EntityType table, numeric type literals, `unsafe`, Rayon, or a
  shared mutable world.
- Do not materialize every generated metadata default into every ECS entity.
  Missing metadata entries continue to mean the generated Vanilla default; the
  sparse canonical representation must stay sparse.
- Do not synthesize custom synchronized EntityType factories. Unknown/custom
  resources remain rejected and diagnosable.
- Do not mark a family complete from a generic zero/default assumption unless
  the generated class/default inventory proves that no client-observable
  specialization exists.

## Owners and Frozen Integration Boundary

- `scripts/entity_metadata_extractor.py` owns Vanilla Java extraction through
  the shared Tree-sitter frontend. It will extend the existing EntityType,
  class graph, metadata-default, and DefaultAttributes inventory instead of
  adding a regex parser or a second catalog generator.
- `scripts/generate-entity-metadata-schema.py` owns deterministic checked-in
  Rust generation. A generated spawn profile is keyed by the existing fixed
  EntityType id/resource and carries exact runtime-class ancestry, attribute
  base values, and mechanically classifiable recreation/default facts.
- `bbb-world::EntityStore` remains the only canonical entity owner. Spawn
  profiles initialize or reset only narrow ECS owners. Renderer/control query
  the same immutable publication and never receive a parallel default table.
- `bbb-protocol` continues to own the fixed EntityType catalog and wire packet;
  no runtime-class or default semantics move into packet decoding. The catalog
  also records the exact `MobCategory` argument from each Vanilla builder. That
  generated field is the authoritative hostile/passive family partition for
  this audit; it is not a hand-maintained membership table and does not alter
  the packet shape.
- `bbb-native` remains an ordered side-effect adapter. Constructor/default state
  that has no Vanilla client side effect does not create a runtime command.

Exact resource strings and generated catalog ids prove identity; hashes may
only accelerate lookup. Generated class/profile tables may be indexed by the
fixed id because admission has already resolved the session holder to that
exact catalog entry.

## Vanilla 26.1 Evidence

Local source root: `/home/zgy/Work/mc-code/sources/26.1`.

- `ClientPacketListener.createEntityFromPacket` constructs `RemotePlayer` only
  from current PlayerInfo; every other accepted type uses `EntityType.create`
  with `EntitySpawnReason.LOAD`.
- `EntityType.create` calls the registered factory and therefore the concrete
  Java constructor before `Entity.recreateFromPacket` applies packet identity,
  position, velocity, and rotations.
- `SynchedEntityData.Builder` installs the `defineSynchedData` defaults during
  construction. bbb's generated metadata schema already records these defaults
  and sparse world queries apply them when no update is present.
- `DefaultAttributes.SUPPLIERS` selects the per-EntityType factory. Chained
  `createAttributes` builders inherit and override exact attribute base values;
  an omitted explicit value uses the registered `Attribute` default.
- `ClientPacketListener.handleAddEntity` runs recreation, inserts into
  `ClientLevel`, starts post-add sounds, and records seen players in that order.
  The finite packet recreation overrides and those side effects are owned by
  the completed AddEntity lifecycle slice, not duplicated here.
- `ServerEntity.sendPairingData` sends non-default synchronized state,
  attributes, equipment/effects/passengers, and other later owners. Those
  packets must override fresh client defaults rather than being guessed at
  spawn.

Critical source facts are rechecked by the main agent before each owning phase;
normal build/test never reads the local Vanilla source tree.

## Serial Phases, Tests, and Gates

1. **Contract and baseline.** Record this contract, the 156-partial baseline,
   existing sparse metadata semantics, and the exact factory/default source
   inventory. Run docs/fmt/diff checks and commit.
2. **Generated spawn profile.** Extend the shared extractor and metadata-schema
   generator with exact 157-entry runtime-class ancestry and per-living-type
   attribute base values, including inheritance/override and registry-default
   handling. Add extractor fixtures, generated update/check tests, uniqueness/
   coverage guards, and source guards against a second table. Run scripts,
   `bbb-world` schema tests, and a milestone full gate; this is the first safe
   stop.
3. **Canonical common defaults.** Replace Turtle-only attribute fallback and
   any generic guessed spawn values with the generated profile. Test all 157
   types, all generated attributes, sparse metadata defaults, same-id re-add,
   registry reorder/custom rejection, disconnect/reset/id reuse, and retained
   publications. Run focused world/native tests, commit, and full-gate.
4. **Family client defaults.** Audit fields actually consumed before their first
   authoritative update, grouped serially as base/non-living, projectile/
   vehicle/display, living/passive, hostile/boss, and Player. Add only narrow
   canonical owners or deterministic client-random state that has a proven
   consumer. Each group is a buildable normal commit with family-focused tests;
   run a full gate at every fourth commit and before crossing a new state owner.
5. **Coverage closeout.** Generate per-family evidence, flip only the
   `spawn/default` cells, remove the matching unsupported todo, and archive the
   result without changing adjacent cells. Run generator checks, coverage
   tests, affected-crate tests, docs/fmt/diff checks, and commit.
6. **Delivery.** At final committed HEAD run `scripts/cargo-dev.sh gate`, build
   `bbb-native` in `/tmp/bbb-target-main`, verify the binary is current, run the
   explicit Weston-backed `scripts/playable-gate.sh live`, then the policy auto
   gate.

Focused validation includes extractor unit tests, generated schema guards,
`bbb-world` entity spawn/default and publication tests, protocol AddEntity
decode/admission tests, native generation/effect ordering tests, and the entity
coverage ledger tests. No wall-clock threshold becomes a CI assertion.

Failures localize to Java extraction, generated profile semantics, common ECS
initialization/reset, one family group, or coverage evidence. A generated
profile or common-default milestone is not crossed until it is committed and
full-gated. Family groups never share knowingly broken intermediate state.
Fix forward with normal commits; never amend, rebase, reset, or rewrite history.
