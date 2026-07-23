# Entity Event Packet Closure Engineering Contract

This document freezes the high-risk engineering slice that closes
`CLIENTBOUND_ENTITY_EVENT` for Vanilla 26.1. The generated declaration inventory,
super-dispatch rules, all 66 direct/delegated declaration-event pairs, the three
`ClientPacketListener` special ids, and their existing canonical, animation,
particle, and focused audio consumers are already reverse-covered. The remaining
gap is `LivingEntity.handleEntityEvent(3)`: its full death-sound/volume matrix and
the client-observable portion of the virtual `die(generic)` transition.

The latest clean baseline is commit `fa197e250`. Its full gate passed all six
steps, and its playable auto gate passed all eight fixture phases. The explicit
live phase was unavailable because no Java executable was configured.

## Goal and Completion

An event packet must first resolve through the generated runtime-class event
handler graph. Only an event that resolves to `LivingDeath` may execute this
path:

```text
CLIENTBOUND_ENTITY_EVENT id 3
  -> fixed entity type + generated handleEntityEvent dispatch
  -> generated getDeathSound/getSoundVolume policy
  -> death sound submission
  -> non-Player health/death lifecycle mutation
  -> immutable WorldStore publication
```

The slice is complete when every fixed living EntityType resolves its nearest
Vanilla `getDeathSound()` and `getSoundVolume()` behavior; event 3 preserves
Vanilla sound-before-death ordering; the non-Player first-death transition
retains its client-observable base and subclass state; Player and non-living
event-3 behavior remain distinct; recreation, reset, serialization, and
publication isolation are tested; the packet's five coverage cells are
complete; its open audio/TODO wording is removed or narrowed to genuinely
remaining ambient/runtime work; this contract is archived; and the final full
and playable gates pass.

## Non-Goals

- Do not infer a death from health, DamageEvent, HurtAnimation, or another
  packet. This slice handles only an EntityEvent that source-derived dispatch
  classifies as `LivingDeath`.
- Do not perform server-owned loot, experience, statistics, advancements,
  reputation, AI, raid, owner notification, combat-score, game-event, entity
  removal, or respawn behavior.
- Do not make event 3 kill a Player. Vanilla plays the Player death sound but
  deliberately skips `setHealth(0)` and `die(generic)` in this handler.
- Do not let Snowball, ThrownEgg, Firework, or another non-living event-3
  declaration fall through to the LivingEntity handler.
- Do not add a handwritten EntityType switch, parallel death-sound table,
  bootstrap registry ordinal, wall-clock input, or in-game configuration
  surface.
- Do not broaden the slice into ambient/step/eat loops, authoritative menu
  closure, death-screen handling, server entity removal, or unrelated entity
  model work.

## Owners and Frozen Integration Boundary

- `bbb-protocol` keeps the existing signed-byte packet wire shape and fixed
  EntityType/sound-event catalogs. No packet decode change is planned.
- `scripts/generate-entity-metadata-schema.py` owns the fixed runtime-class
  reverse inventory. Each living row gains its nearest `getDeathSound()` owner
  and a closed source-interpreted death-sound policy. Unknown method bodies,
  conditions, branch order, or sound constants fail generation.
- `bbb-world::EntityStore` remains the sole canonical entity owner. A narrow
  living-lifecycle component retains Vanilla's client `dead` flag separately
  from synchronized health, because `setHealth(0)` alone does not prove that
  `die()` has run. It is recreated, reset, serialized, cloned, and published
  with the existing entity incarnation.
- `bbb-world::WorldStore` resolves sound policy inputs from canonical metadata,
  registries, fluid state, transform, and generated sound-source/volume policy.
  It resolves and records the sound before the first-death state mutation.
- `bbb-world::apply_play_packet` remains the unique ordered packet/effect owner.
  It invokes the external positioned-sound effect before applying the
  non-Player death transition, matching the source call order; no dispatcher or
  native path may rebuild the event-id switch.
- `bbb-native` remains an effect adapter. It does not own entity death state or
  reinterpret entity type/event ids.

## Vanilla 26.1 Evidence

Local source root: `/home/zgy/Work/mc-code/sources/26.1`.

- `ClientPacketListener.handleEntityEvent` handles ids 21, 35, and 63 specially;
  every other id calls `entity.handleEntityEvent(id)`.
- `LivingEntity.handleEntityEvent(3)` calls `getDeathSound()`, then
  `playSound(sound, getSoundVolume(),
  (random.nextFloat() - random.nextFloat()) * 0.2F + 1.0F)`. Afterwards, only
  for non-Player instances, it calls `setHealth(0)` and
  `die(damageSources().generic())`.
- `LivingEntity.die` runs only while the entity is not removed and `dead` is
  false. It stops sleeping and item use, sets `dead = true`, and sets
  `Pose.DYING`. Its `ServerLevel` branch owns loot, game events, and event
  broadcast and is excluded here.
- `LivingEntity.stopSleeping` clears an occupied loaded bed when present,
  resolves the safe stand-up position/yaw (or the above-bed fallback), returns
  through standing pose, and clears `sleepingPos`; `die` then replaces that
  temporary standing pose with `DYING`.
- Client-observable virtual `die` overrides are narrow: `Sniffer` transitions
  synchronized state to `IDLING`; `Wolf` clears wet/shaking interpolation before
  delegating. `AbstractVillager.stopTrading` clears a client entity pointer that
  bbb does not currently create; its active menu remains packet-owned. Raider,
  TamableAnimal, Villager, and IronGolem add only server-owned/no-op behavior
  before or after the base transition.
- The generated reverse inventory over all 157 fixed EntityTypes finds 92
  living rows and 88 distinct nearest `getDeathSound()` owners. Fixed returns
  cover 71 owners. The 17 dynamic owners are cat/chicken/abstract-cow/pig/wolf
  synchronized sound variants; Horse/HappyGhast/Turtle age; Goat screaming;
  Slime/MagmaCube size; Drowned/Guardian/ElderGuardian water; ZombieNautilus
  underwater; Nautilus age plus underwater; and CopperGolem oxidation.
- The already-generated `getSoundVolume()` inventory is shared unchanged:
  fixed exact `f32` values except Slime/MagmaCube (`0.4F * size`) and
  HappyGhast (`1.0F` baby, `4.0F` adult).

The main agent rechecks each dynamic method body and the first-death subclass
overrides before its owning implementation stage. Ordinary builds and tests use
only checked-in generated Rust and never read the external source tree.

## Frozen State and Audio Contract

The generated death policy has the closed shapes `NonLiving`, fixed resource,
condition with true/false resources, Nautilus age/water, CopperGolem oxidation,
and sound-variant family. Its condition vocabulary reuses the canonical
metadata/fluid predicates already required by generated hurt audio. Wolf death
uses the synchronized sound variant without the hurt-only armor branch. Every
fixed resource must exist in the protocol sound catalog.

Sound suppression checks the canonical silent flag before consuming RNG.
Successful playback consumes exactly two deterministic level-RNG float samples
in subtraction order, uses generated sound source and volume, and records seed
zero with no distance delay. Missing entity, non-living dispatch, unknown
registry identity, or unresolved dynamic state consumes no samples.

After sound submission, non-Player state applies in Vanilla order. Health
becomes zero. Wolf/Sniffer pre-super hooks run on every virtual `die` call
because their overrides sit outside `LivingEntity.die`'s `dead` guard. On the
first call for an incarnation, the guarded base path then performs the sleeping
transition and modeled item-use stop, marks the lifecycle dead, and sets pose
`DYING`; post-super hooks run afterwards where bbb has a canonical consumer. A
repeated event still plays the death sound, writes health zero, and reruns the
two pre-super overrides, but does not rerun the guarded base transition. Player
event 3 plays sound only. Same-id recreation and session/level reset start a
fresh non-dead incarnation.

## Serial Stages, Tests, and Gates

1. **Source policy milestone.** Extend the generator, schema type, generated
   snapshot, and reverse guards with all 92 living death policies and 88 owner
   identities. Add extractor tests for every dynamic shape and drift failure.
   Run generator unit tests, generator `check`, generated-schema focused tests,
   formatting, diff check, and complete review. Commit and run the full gate
   because this stage changes a generator. This gated commit is the first safe
   stop.
2. **Canonical first-death lifecycle.** Add the narrow lifecycle owner and exact
   non-Player first-death transition, including sleeping/pose and Sniffer/Wolf
   client hooks. Test Player separation, non-living dispatch, repeat handling,
   recreation/reset, serialization/publication, and scheduled death animation.
   Run affected `bbb-world` focused tests and all per-stage checks, then commit.
3. **Audio and packet closure.** Replace the handwritten death-sound switch with
   generated policy consumption, preserve sound-before-state ordering and
   two-sample RNG, add exhaustive world/native tests, close the packet ledger
   and TODO, archive this contract, and review the integrated diff. Run protocol
   coverage, affected world/native tests, both generator checks, and all
   per-stage checks. Commit, run the full gate, then playable live explicitly
   and playable auto according to repository policy.

A stage failure is fixed forward within the same stage. Generator failures are
localized by extractor tests and generated diff; lifecycle failures by focused
EntityEvent/state/publication tests; audio failures by exhaustive policy and
ordered native effect fixtures. No later stage starts until the preceding
commit is clean. If all 92 rows cannot be classified or an exact first-death
input lacks a canonical owner, the safe stop is the last fully gated milestone
with the packet row left partial.
