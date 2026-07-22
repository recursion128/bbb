# Entity Event, Particle, and Audio Closure

This document freezes the engineering contract for closing the Vanilla 26.1
`event/particle/audio` coverage cell for all 157 fixed entity types. Work starts
from full-gated commit `460925cbc`.

## Goal and Completion Standard

Every fixed type must have one source-backed audiovisual disposition:

- every reachable `handleEntityEvent(byte)` declaration and the three
  `ClientPacketListener` special ids resolve through the generated class graph
  to exactly one semantic world consumer and every particle, sound, animation,
  or canonical mutation performed by that handler is preserved in Vanilla
  order;
- every client-side subclass `tick`, `baseTick`, or `aiStep` particle/audio
  effect either drains from the existing typed client-tick report before world
  publication or has a source-backed server-only / no-effect classification;
  and
- packet- and tick-owned particles reach the native particle runtime while
  sounds reach the existing audio event sink, with deterministic client-local
  RNG and explicit resource fallbacks.

The category closes only after the generated event and tick inventories cover
all 157 rows, the confirmed gaps below are removed, the matrix contains no
`partial` or `deferred` cell, and focused tests, the full gate, and
`scripts/playable-gate.sh` pass at current HEAD.

## Non-Goals

- Equipment applicability, passenger/control rules, and leash presentation
  remain in `equipment/passenger/leash`.
- Level-event packets, explosion-packet presentation, particle-provider tick
  mechanics, block-entity idle effects, block/block-entity crumbling, and the
  physical audio device remain in their existing ledger owners. Their already
  shared particle/audio runtime may be reused but is not reclassified as entity
  family behavior.
- Server AI, combat results, damage, splitting Slimes, projectile hits,
  explosions, authoritative removal, and server-only sound broadcasts are not
  simulated.
- Client-local presentation RNG is deterministic for fixtures but is never
  promoted to server-authoritative canonical state.

## Owners and Frozen Boundaries

- `bbb-protocol` owns the decoded `ClientboundEntityEventPacket`, fixed entity
  ids, particle ids, and sound-holder wire shapes. No packet format change is
  planned.
- `bbb-world::entities::schema` owns the source-generated runtime class graph,
  all 35 `handleEntityEvent` declarations, direct/delegated ids, and superclass
  reachability. `entities::events` is the only handwritten semantic naming
  boundary; no second entity-type or raw event-id switch may be added.
- `bbb-world` owns canonical event mutations, client animation/tick state,
  deterministic RNG cursors, and narrow immutable particle/sound reports.
  `WorldStore::apply_play_packet` remains the single packet apply path.
- `bbb-native` drains `PlayApplyEffects` and entity client-tick reports before
  publication, resolves particle templates/item/block context, and submits
  audio events. It does not reinterpret entity families or rerun world logic.
- `bbb-renderer` and the audio backend consume typed submissions only. They do
  not query live entity state or own event dispatch.

## Vanilla 26.1 Evidence

The main agent rechecks local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each correction. Primary anchors
are `ClientPacketListener.handleEntityEvent`, every generated
`handleEntityEvent(byte)` source, and the nearest generated `tick`, `baseTick`,
and `aiStep` owners for each runtime class.

The first reverse audit confirmed:

- the generated catalog already owns 35 handler declarations, superclass
  forwarding, the `MinecartSpawner` delegated event, and all declaration ids;
  the handwritten semantic enum maps every generated id, but the coverage
  evidence currently cites only the generic apply method;
- `Rabbit.handleEntityEvent(1)` calls `spawnSprintParticle()` before seeding its
  hop clock. The hop is covered; the block particle is not;
- `Hoglin` and `Zoglin` event `4` call `makeSound` after arming the attack timer.
  `Creaking` event `4` calls its attack sound and event `66` calls the ordinary
  hurt-sound path. Their animation timers are covered but these sounds are not;
- `Slime.tick` emits `width * 2 * 16` item-slime particles and its size-scaled
  squish sound on the airborne-to-ground transition before updating
  `targetSquish`. `MagmaCube` inherits the ordering while replacing the particle
  with flame and selecting magma-cube sounds. Existing code advances only the
  squish state; and
- Firework event `17` already projects explosion component data into the native
  Starter implementation, including shape/color/fade/trail/twinkle, near/far
  audio and empty fallback. Its client tick already owns launch audio and the
  rocket trail. Closure therefore needs reverse evidence and regression tests,
  not a parallel firework implementation.

Historical ledger wording is not evidence. Any additional rejected family is
verified against its current source before editing.

## Ordered Stages and Focused Tests

### A. Exhaustive event consumers

Add an exhaustive semantic disposition check on top of the generated handler
catalog and listener specials. Complete Rabbit's sprint block particle and the
Hoglin/Zoglin/Creaking sound consumers through the existing packet apply and
runtime sinks. Preserve handler ordering and silent/baby pitch rules.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world entity_event
scripts/cargo-dev.sh test -p bbb-world transient_events_and_animation_triggers
scripts/cargo-dev.sh test -p bbb-native entity_particle_events
scripts/cargo-dev.sh test -p bbb-native particle_runtime
```

### B. Client-tick audiovisual closure

Extend the existing typed client-tick trigger/report path for Slime and Magma
Cube landing effects. Keep the ground transition, RNG draws, particle batch,
sound, and squish mutation in source order. Reverse-check that every generated
runtime tick owner is already covered by the closed client-tick engineering
inventory or receives a narrow server-only/no-effect disposition; do not add a
second tick table.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world small_mob_and_walk_animation
scripts/cargo-dev.sh test -p bbb-world entity_client_tick
scripts/cargo-dev.sh test -p bbb-native particle_runtime
scripts/cargo-dev.sh test -p bbb-native client_tick
```

### C. Coverage closeout and delivery

Expand `entity_event_audit` with the generated dispatch, typed event/tick
reports, native particle/audio consumers, semantic tests, and Vanilla anchors.
Prove Firework event/tick ownership, remove the closed TODOs, regenerate the
157-row matrix, and append the engineering milestone index. No entity
`event/particle/audio` cell may remain `partial` or `deferred`.

Run:

```text
scripts/cargo-dev.sh gate
scripts/playable-gate.sh
```

## Checkpoints, Failures, and Safe Stops

The baseline is fully gated. The contract, exhaustive event correction,
Slime/Magma Cube tick correction, and coverage closeout are separate buildable
commits. Run the full gate after at most four commits and at final closeout. A
focused failure is fixed forward; `batch-gate` is used only when the introducing
commit is unclear.

Safe stops are a clean contract checkpoint, a fully wired packet-event
correction, a fully wired client-tick effect correction, or a documented
external blocker with prior behavior intact. World reports never land without
their native particle/audio consumer in the same checkpoint.
