# Damage Event Packet Closure Engineering Contract (Archived)

This document freezes the high-risk engineering slice that closes
`CLIENTBOUND_DAMAGE_EVENT` for Vanilla 26.1. Decode, session `DamageType`
identity, canonical source retention, the ten-tick hurt overlay, player
effect-selected audio, and five animal sound-variant families already exist.
The remaining gap is the rest of `LivingEntity.handleDamageEvent`: living-only
dispatch, walk-animation impulse, invulnerability and last-source timing, and
the exhaustive non-player hurt-sound/volume matrix.

The latest clean baseline is commit `f5370c841`. Its full gate passed all six
steps, and its playable auto gate passed all eight fixture phases (the live
phase was unavailable because Java was not installed).

## Goal and Completion

An ordered DamageEvent must resolve its source through the current session
registry and then execute the exact client-observable Vanilla living-entity
handler before its hurt sound is submitted:

```text
CLIENTBOUND_DAMAGE_EVENT
  -> session DamageType identity/effects
  -> living-only canonical EntityStore mutation
  -> walk speed + invulnerability/hurt timers + last source/stamp
  -> generated runtime-class hurt-sound and volume policy
  -> ordered native positioned-sound sink
  -> immutable WorldStore publication
```

The slice is complete when all fixed living EntityTypes use a source-derived
`getHurtSound` and `getSoundVolume` policy; non-living and missing entity ids are
no-ops; canonical timing, reset/id-reuse, and retained-publication isolation are
tested; runtime sound ordering is tested; the packet's five coverage cells are
complete; its open TODO is removed; and the final full and playable gates pass.

## Non-Goals

- Do not infer health loss, absorption, death, knockback, combat outcome,
  immunity, or server-authoritative damage acceptance from this packet.
- Do not port server damage calculation, armor/enchantment rules, AI reactions,
  goals, loot, statistics, advancement, or entity removal.
- Do not use `invulnerableTime` to reject a later DamageEvent. The packet is an
  authoritative client presentation notification, and Vanilla handles each one.
- Do not add a second entity-type table, numeric type-id switch, audio-owned
  mutable entity state, wall-clock timestamp, or in-game configuration surface.
- Do not broaden the packet into entity-event death audio, ambient audio, voice
  pitch, or renderer/model work unrelated to the existing hurt overlay and walk
  accumulator.

## Owners and Frozen Integration Boundary

- `bbb-protocol` keeps the existing packet wire type, DamageType effect enum,
  fixed EntityType catalog, and sound-event catalog. No wire change is planned.
- `scripts/generate-entity-metadata-schema.py` extends the existing source-owned
  runtime-class table. For every fixed EntityType it emits a hurt-sound policy
  and sound-volume policy derived from the nearest Java method owner. Fixed
  `SoundEvents` constants are resolved to resource ids through
  `sound_event_catalog_extractor.py`; unrecognized method bodies fail generation.
- `bbb-world::EntityStore` remains the sole canonical owner of the current
  living damage presentation state. `EntityDamage` retains the last source and
  level game-time stamp plus the remaining invulnerability ticks. The existing
  `EntityClientAnimationState` owns hurt time and `WalkAnimationState`.
- `bbb-world::WorldStore` resolves dynamic sound inputs from canonical metadata,
  equipment, registries, transform/bounds, and loaded fluid state. Its audio
  resolver consumes the generated policy; it does not establish family identity
  with a parallel handwritten type list.
- `bbb-native` remains an ordered effect adapter. It receives at most one
  positioned sound after the world mutation and does not reread or mutate damage
  state independently.

Exact resource identity is authoritative. Generated table indexing is allowed
only after fixed EntityType admission has established that identity.

## Vanilla 26.1 Evidence

Local source root: `/home/zgy/Work/mc-code/sources/26.1`.

- `ClientPacketListener.handleDamageEvent` looks up the packet entity and calls
  `entity.handleDamageEvent(packet.getSource(level))`; the base `Entity` method
  is empty.
- `LivingEntity.handleDamageEvent` sets walk-animation speed to `1.5F`,
  `invulnerableTime` to `20`, `hurtDuration` and `hurtTime` to `10`, selects and
  plays `getHurtSound(source)` at `getSoundVolume()` and
  `(random.nextFloat() - random.nextFloat()) * 0.2F + 1.0F`, then stores the
  source and `level.getGameTime()` stamp.
- `LivingEntity.baseTick` decrements positive `hurtTime` and client-side
  `invulnerableTime`. `getLastDamageSource` expires the retained source only
  when `level.getGameTime() - lastDamageStamp > 40`.
- `WalkAnimationState.setSpeed` changes only current `speed`; the next ordinary
  living client tick continues through the existing walk-animation update.
- The generated reverse inventory over all 157 fixed EntityTypes finds 92
  living types, 89 distinct nearest `getHurtSound` owners, and 13 nearest
  `getSoundVolume` owners. Fixed returns cover most types. Dynamic cases are:
  player DamageType effects; cat/chicken/cow/pig/wolf sound variants; wolf armor
  bypass tags; slime/magma size; guardian, elder guardian, drowned, nautilus,
  and zombie nautilus water state; shulker closure; armadillo scared state; goat
  screaming state; horse, happy-ghast, nautilus, and turtle age; copper-golem
  oxidation; and Creaking heart binding.
- Sound volume is fixed except slime/magma (`0.4F * size`) and Happy Ghast
  (`1.0F` baby, `4.0F` adult). Fixed overrides include bat `0.1F`,
  allay/bee/cow/squid/wolf `0.4F`, horses `0.8F`, Warden `4.0F`, and Ghast/
  Ender Dragon `5.0F`.

The main agent rechecks each dynamic method body and registry/equipment branch
before its owning implementation stage. Ordinary builds and tests consume only
the checked-in generated Rust snapshot and never read the local source tree.

## Frozen State and Audio Contract

DamageEvent admission first resolves the entity's fixed type and returns false
for missing or non-living entities. A successful event mutates the existing
owners in Vanilla order: current walk speed becomes `1.5`, invulnerability time
becomes `20`, hurt time becomes `10`, and the exact packet source plus current
level game-time stamp replaces the previous last source. Registry projection
errors remain diagnosable and do not invent a different DamageType identity.

Only normally scheduled living client ticks decrement invulnerability and hurt
time. The public last-damage query applies Vanilla's strict `> 40` expiry
against current level game time without erasing retained historical
publications. Serialization, same-id recreation, type reuse, level/session
reset, and immutable publication cloning carry or clear the new fields through
the existing component lifecycle.

The generated policy has two closed enums: hurt sound (`NonLiving`, fixed
resource, player/variant/special dynamic kinds) and volume (fixed exact `f32`
bits, slime size, or Happy Ghast age). Generator guards prove all 92 living rows
have a sound and volume policy and every fixed sound resource exists in the
protocol sound catalog. Runtime dynamic resolution uses existing canonical
metadata accessors and registry variant projections. Water-dependent branches
use the same loaded-world AABB/fluid fact used by living render extraction;
missing chunk/fluid data follows the existing dry/default-false path.

Silence suppresses sound only; it does not suppress canonical damage state.
Sound position/source use the canonical entity transform and generated sound
source policy. Pitch consumes exactly two deterministic client level-RNG
samples in subtraction order. The native effect is submitted after canonical
mutation and before the resulting publication can be observed.

## Serial Stages, Tests, and Gates

1. **Source policy milestone.** Extend the generator and checked-in schema with
   exhaustive sound/volume policy extraction and drift guards. Run the generator
   unit tests, generator `check`, focused generated-schema world tests,
   formatting, diff check, and complete review. Commit and run the full gate
   because this stage changes a generator. This gated commit is the first safe
   stop.
2. **Canonical living state.** Add living-only admission, exact walk/
   invulnerability/hurt/last-source timing, component serialization/reset/
   publication behavior, and focused status/animation/lifecycle tests. Run
   affected `bbb-world` tests plus all per-stage checks and commit.
3. **Audio and runtime closure.** Consume the generated policies, implement all
   fixed and dynamic sound/volume cases, preserve two-sample RNG and effect
   ordering, add exhaustive world and native runtime tests, then close the
   coverage ledger/TODO and archive this contract. Run protocol coverage,
   affected world/native tests, generator `check`, and all per-stage checks.
   Commit, run the full gate, then run playable live explicitly and playable
   auto according to repository policy.

A stage failure is fixed forward within the same stage. Generator failures are
localized by its Python unit tests and generated diff; state failures by
DamageEvent/status/animation lifecycle filters; audio failures by per-policy
world tests and the native ordered-effect fixture. No later stage starts until
the preceding commit is clean. If the source inventory cannot classify every
fixed living row, or a required dynamic input lacks a canonical owner, the safe
stop is the last fully gated milestone with the coverage row left partial.
