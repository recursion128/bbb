# Block-entity client effects closeout engineering contract (archived)

## Goal and completion

Close the remaining idle/tick block-entity client-effects ledger item by
running every Vanilla 26.1 block-entity client ticker that has a locally
observable particle or audio side effect through one deterministic
world-to-native effects path.

The engineering effort is complete when:

- lit campfires run `CampfireBlockEntity.particleTick`, including the 11%
  smoke burst, signal/cosy smoke choice, and occupied-slot smoke positions;
- ordinary mob spawners emit one smoke and one flame particle on every active
  nearby-player tick that advances their display spin;
- trial spawners emit state-specific idle particles and the normal/ominous
  ambient sound only in states capable of spawning;
- conduits emit frame and synchronized destroy-target nautilus particles after
  their client state advances;
- vaults advance their display spin clock, emit idle smoke/flame, emit
  20-tick connected-player particles, and play the active-display ambient
  sound with the exact client-local random order;
- all five owners consume the same non-serializable `ClientLevel.random`
  cursor already used by native animate-tick effects, reset on the same
  connection/level-generation edge, and execute before
  `ClientLevel.animateTick`;
- missing particle or audio sinks still consume the exact random stream and
  never replay effects when a sink later appears;
- world tests prove bounded typed NBT/state projections and ticker gates,
  native tests prove exact random order plus particle/audio commands, and a
  runtime-order test proves block-entity effects precede animate tick and
  particle-engine advancement; and
- the corresponding todo is removed from
  `docs/unsupported-features.md`, this contract is indexed in
  `docs/goal-archive.md`, and the final HEAD passes the full gate.

## Non-goals

- Bell resonation particles and raider glow; that behavior remains owned by
  the separate raid-feature decision in the ledger.
- Trial-spawner custom `SpawnData` display-entity NBT or general synthetic
  entity loading, which remains deferred under block entities and terrain.
- Server-owned beacon/conduit effects, trial-spawner state transitions,
  spawning, loot, damage, or authoritative world mutation.
- Block `animateTick` effects such as ender-chest/end-gateway portal particles
  and campfire crackle/lava; the existing environment animate owner remains
  authoritative for them.
- Reimplementing packet-owned level events 3011 through 3021. Tick effects
  consume current canonical block/block-entity state and do not synthesize
  level events.
- A second particle resolver, audio backend, random cursor, block-entity
  object graph, or per-frame world-to-render setter.

## Ownership and frozen integration surface

### `bbb-world`

`bbb-world` owns bounded semantic projections of the server-supplied
block-entity fields needed by the client tick:

- campfire occupied slot indices;
- conduit destroy-target UUID;
- vault display-item presence, connected-player UUIDs, and connection range.

A single `WorldStore::block_entity_client_effect_sources` query is the
authoritative projection for the five ticker families. Its typed source enum
contains only canonical positions, block properties, decoded block-entity
facts, current client ticker state, and resolved entity targets. It is sorted
deterministically by block position and ticker family. It does not contain a
random generator, backend command, particle sprite, audio handle, or runtime
queue.

Existing per-family state owners continue to advance model clocks. The unified
query delegates to those owners instead of reconstructing spawner/conduit
ticker gates in native code.

### `bbb-native`

`EnvironmentAnimateRuntime` remains the sole owner of the native
`ClientLevel.random` cursor. A named block-entity-effects step consumes the
typed world sources once per running level tick, appends structured
`EnvironmentParticleSpawn` values, and submits local sounds through the
existing effect collector.

Particle options are resolved by the existing `ParticleEventSink` /
`spawn_environment_particles` path. Audio commands use the existing
`LocalSoundEventState` path. Sink absence changes only submission, never
canonical state advancement or random consumption.

The runtime call order is frozen as:

1. advance block-entity canonical/model ticker state;
2. derive and consume block-entity client effects;
3. run the existing environment `animateTick`;
4. advance ordinary particle-engine state.

### `bbb-renderer` and `bbb-audio`

No new backend surface is introduced. The renderer consumes ordinary resolved
particle batches, while audio consumes existing positioned local-sound
commands. Resource-pack lookup, limiter behavior, attenuation, captions, and
missing-resource diagnostics stay in their current owners.

## Vanilla 26.1 authority

- `CampfireBlock.getTicker`, `CampfireBlockEntity.particleTick`, and
  `CampfireBlock.makeParticles`.
- `SpawnerBlock.getTicker` and `BaseSpawner.clientTick`.
- `TrialSpawnerBlock.getTicker`, `TrialSpawner.tickClient`, and
  `TrialSpawnerState.ParticleEmission`.
- `ConduitBlock.getTicker` and `ConduitBlockEntity.clientTick` /
  `animationTick`.
- `VaultBlock.getTicker`, `VaultBlockEntity.Client.tick`, and
  `VaultSharedData`.
- `ClientLevel.tickBlockEntities`, `ClientLevel.animateTick`, and
  `ClientLevel.playLocalSound` for ordering and client-local random/audio
  semantics.

All references are read from the local
`~/Work/mc-code/sources/26.1/` source tree.

## Serial stages and verification

1. **Typed world projections**
   - Decode the three bounded NBT facts, add the unified source query, and
     delegate ordinary-spawner/conduit gates to existing ticker owners.
   - Focused tests:
     `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-world block_entity_client_effect`
     plus campfire, spawner, conduit, trial-spawner, and vault filters.
2. **Native deterministic effects**
   - Add the named block-entity effects step to
     `EnvironmentAnimateRuntime`, preserving one random cursor and the five
     exact Vanilla algorithms.
   - Focused tests:
     `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-native block_entity_client_effect`
     plus environment-animate and particle-runtime filters.
3. **Integration and closeout**
   - Prove effect ordering/sink independence, remove the ledger item, index the
     engineering closure, review the complete diff, and run all per-commit
     checks followed by `scripts/cargo-dev.sh gate`.

Each stage is a normal, buildable commit. The fully gated `fc3d961dd` is the
pre-start checkpoint. Stage commits are safe stopping points only after their
focused tests and per-commit checks; the final HEAD must pass the full gate
before another slice starts. Failures are fixed forward without rewriting
history.
