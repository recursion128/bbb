# Dynamic RegistryAccess typed-revision engineering contract

## Semantic goal and completion criteria

Build one world-owned, atomically committed typed revision for the 28 vanilla
26.1 configuration-synchronized registries. A successful configuration finish
must expose one mutually consistent holder order, source-neutral element data,
typed client projections, synchronized and static tags, declared validators,
and registry-dependent item default components. A failed element decode,
cross-registry reference, tag application, validator, or component initializer
must leave the previously frozen revision and its item defaults unchanged.

The engineering slice is complete when:

- every synchronized registry element is decoded from network NBT or selected
  known-pack JSON before commit, and every client-consumed typed family records
  either a validated typed value or an entry-scoped codec failure;
- holder and tag references used by those typed values resolve against the
  candidate revision, never the prior revision or vanilla bootstrap order;
- all validators declared by the generated synchronized registry inventory run
  against the candidate contents before publication;
- static tags and synchronized tags are materialized against the same candidate
  holder view, with vanilla's unresolved-holder omission retained as a separate
  diagnostic;
- the registry-dependent outputs of vanilla's item data-component initializers
  used by bbb are rebuilt from that candidate access and published with it;
- `FrozenRegistryAccess` exposes only a fully committed revision, and session
  reset, same-session reconfiguration, transfer, and failed reconfiguration do
  not leak an incompatible typed/tag/component generation;
- custom insertion/reordering, missing/stub data, malformed values, duplicate
  packets, static and synchronized tag updates, reconnect, and rollback have
  focused end-to-end coverage; the registry and packet ledgers describe the
  remaining concrete Codec/consumer gaps instead of a generic typed-revision
  lifecycle gap.

## Non-goals

- Do not reproduce Mojang's generic `Codec`, `RegistryAccess`, Java registry
  class hierarchy, or server/world-generation behavior.
- Do not eagerly implement typed values for registries without a current client
  consumer merely for abstract completeness; retain their bounded,
  source-neutral element value until a consumer owns a narrow projection.
- Do not change fixed built-in registry authority or allow a synchronized
  registry to fall back by vanilla ordinal after the server supplied contents.
- Do not include resource-pack reload, full item-component semantic closeout,
  entity-family presentation, or the complete ChatListener slice.

## Ownership and integration contract

- `bbb-protocol` owns the generated 28-entry synchronized inventory and raw
  packet/NBT shapes. It does not own session registry state or typed client
  values.
- `bbb-world::registries` owns pending collection, candidate construction,
  typed projections, tags, validators, registry-dependent item defaults, and
  the single committed revision. Existing family modules retain their semantic
  value decoders; a new revision module only orchestrates them and exposes a
  narrow candidate/commit facade.
- `bbb-net` and probe/native dispatch continue to call the one
  `finish_registry_configuration` boundary. A returned error queues the
  existing configuration disconnect path; callers never perform partial
  recovery or rebuild a second registry switch.
- `bbb-native`, `bbb-item-model`, renderer, audio, and control consume immutable
  world projections. They must not cache a holder order independently or read
  pending configuration state.
- Ordering is fixed as: materialize network/known-pack values -> construct the
  candidate holder context -> resolve candidate tags -> decode typed values and
  holder/tag references -> validate registries -> build registry-dependent
  components -> publish contents/typed/tags/components and revision together ->
  refresh derived consumers/counters.
- Missing server element data is legal only when the selected known-pack source
  supplies that exact registry/resource pair. Unknown holder references and
  malformed typed values fail the candidate when vanilla's Codec would fail;
  unresolved ids inside network tags are omitted from the checked tag view and
  retained diagnostically, matching `NetworkPayload.resolve`.

## Vanilla 26.1 evidence

- `ClientConfigurationPacketListenerImpl.handleRegistryData` and
  `handleUpdateTags` only append to `RegistryDataCollector`; the switch occurs
  from `handleConfigurationFinished`.
- `RegistryDataCollector.loadNewElementsAndTags` builds the complete network
  load set, separates synchronized from static tags, and calls
  `RegistryDataLoader.load` with `SYNCHRONIZED_REGISTRIES`.
- `NetworkRegistryLoadTask.load` decodes every supplied element using the
  registry's declared element Codec and a shared `RegistryOps` context, then
  registers elements and binds its network tags.
- `RegistryDataLoader.load` waits for all registry tasks, freezes them, runs each
  declared `RegistryValidator`, and publishes only if the shared error map is
  empty.
- `RegistryDataCollector.collectGameRegistries` freezes the composite access,
  applies pending static tags, then builds/applies
  `BuiltInRegistries.DATA_COMPONENT_INITIALIZERS` against that frozen access.
- `MappedRegistry.freeze` binds values, rejects unbound entries/tags, builds the
  component lookup, and refreshes tag membership before the registry becomes
  readable.

Authoritative files are under the local vanilla 26.1 tree:

- `net/minecraft/client/multiplayer/ClientConfigurationPacketListenerImpl.java`
- `net/minecraft/client/multiplayer/RegistryDataCollector.java`
- `net/minecraft/resources/RegistryDataLoader.java`
- `net/minecraft/resources/NetworkRegistryLoadTask.java`
- `net/minecraft/resources/RegistryLoadTask.java`
- `net/minecraft/core/MappedRegistry.java`
- `net/minecraft/core/component/DataComponentInitializers.java`
- `net/minecraft/tags/TagNetworkSerialization.java`
- `net/minecraft/world/clock/WorldClock.java`

## Serial phases and validation

### Phase 1: atomic typed candidate owner

Introduce the candidate typed-revision owner, move configuration validation and
tag materialization behind it, and publish only after all registered typed
decoders and declared validators succeed. Preserve the previous frozen access
and item defaults on every failure. Cover network/known-pack equivalence,
cross-registry candidate lookup, duplicate/malformed/unknown reference failure,
static-tag versus synchronized-tag ordering, rollback, same-session
reconfiguration, and new-session reset.

The first registered typed guard is `WorldClock.DIRECT_CODEC`, whose vanilla
unit map codec provides a narrow proof that semantic decode failures occur
inside candidate construction. Phases 2-4 extend the same dispatcher for their
owned families; they do not add independent commit paths.

Focused validation: full `bbb-world` registry tests, `bbb-net` configuration
probe tests, and `bbb-native` configuration event tests, plus non-test checks for
all three crates.

Safe stop: the phase commit retains existing consumer results and only changes
when a revision becomes visible. A failure remains localized to candidate
construction; no later typed family is required for rollback correctness.

### Phase 2: environment attributes and timelines

Complete the remaining dimension environment-attribute modifier/value types and
the synchronized visual, audio, and gameplay timeline tracks. Store their typed
values in the phase-1 candidate and migrate all current dimension/sky/audio/game
consumers to the frozen projection. Cover custom holder order, direct/named
references, clocks, track layering, malformed values, and deterministic time
sampling.

Focused validation: `bbb-world` registries/clock/level tests and affected
`bbb-native` renderer/audio projection tests, with their non-test checks.

Safe stop: existing direct fog/lightmap behavior remains the fallback for a
valid revision that has no applicable track; malformed supplied typed data does
not publish.

### Phase 3: sound variants and test environments

Status: complete.

Decode complete animal sound sets and route every client-consumed non-death
sound through the frozen variant projection. Replace permissive test-environment
game-rule pairs with generated built-in key/type dispatch and exact value
validation while retaining custom keys only where vanilla's registry permits
them. Register both families with the phase-1 candidate.

Focused validation: `bbb-world` entity audio/game-test registry tests and
affected `bbb-native` audio/debug/runtime tests, with non-test checks.

Safe stop: death sounds and already strict test-environment fields remain
unchanged; new consumers activate only for a fully validated typed entry.

Implementation evidence:

- the candidate dispatcher now decodes every field of cat, chicken, cow, pig,
  and wolf `DIRECT_CODEC` sound sets, validates every named `SoundEvent.CODEC`
  reference against the candidate/static sound-event layer, and stores the
  resulting adult/baby values in the same typed revision as dimensions,
  timelines, and clocks;
- the generated vanilla fallback catalog owns both adult/baby hurt and death
  events. `DamageEvent` consumes the frozen hurt projection for all five
  families, including cow/wolf volume, wolf-body-armor override, and the
  `bypasses_wolf_armor` damage-type tag; existing event-3 death behavior uses
  the same typed value without a second holder-order cache;
- a generated 59-entry `GameRules.java` catalog owns built-in keys, boolean
  codecs, and exact integer ranges. Test-environment game-rule maps reject
  unknown keys, JSON boolean/integer type mismatches, and out-of-range values;
  retained read-only known-pack JSON type evidence prevents JSON `true` from
  being confused with numeric `1` after the common NBT numeric projection;
- malformed sound references or game rules fail
  `registry_configuration_typed_values` before publication, with focused
  rollback proofs that retain the prior holder order, revision, and typed
  environment.

### Phase 4: component initializers and closeout

Rebuild the registry-dependent item defaults currently consumed by bbb
(`damage_resistant`, `jukebox_playable`, `blocks_attacks`,
`provides_banner_patterns`, and `instrument`) against candidate tags/holders,
then publish them with the registry revision. Add reverse guards against lazy
pending reads and independent holder-order caches, register the remaining
currently consumed typed families with the candidate dispatcher, migrate
ledgers/docs, and retain custom-reorder proofs for damage/dimension, item/trim,
audio/jukebox, biome, and entity variants.

Focused validation: full affected `bbb-world`, `bbb-item-model`, `bbb-native`,
and `bbb-pack` tests plus non-test checks. Run the full workspace gate and
`scripts/playable-gate.sh auto` at this high-risk milestone HEAD.

Safe stop: each normal phase commit is buildable and reviewable. If a later
consumer exposes a malformed or missing typed family, fix it forward in that
family phase; do not publish a partially rebuilt component generation or
rewrite earlier phase history.

## Gate positions

Every phase runs the per-commit checks from `goal.md`. This changes a core
configuration/session architecture boundary, so phase 1 and the final phase are
forced full-gate checkpoints; phase 2 or 3 also forces a full gate if focused
tests reveal cross-family behavior. The delivered final HEAD must pass the full
workspace gate and playable gate.
