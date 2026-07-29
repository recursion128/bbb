# Synchronized Enchantment Client-Effect Closure

This document freezes the engineering contract for the remaining synchronized
enchantment-effect TODO in `docs/unsupported-features.md`. Work starts from the
fully gated commit `b97aab136`.

## Goal and Completion Standard

Close every client-observable Vanilla 26.1 consumer of synchronized
`EnchantmentEffectComponents`:

- retain the existing synchronized definition, tooltip, attributes,
  `prevent_armor_change`, trident sound, deterministic crossbow charge-time,
  and trident spin-strength behavior;
- decode and execute all six `EnchantmentValueEffect` variants, including
  exact `remove_binomial` small-sample and Gaussian branches;
- give each living item owner a mutable LegacyRandomSource cursor rather than
  fabricating a constant or stateless random result;
- use synchronized crossbow charge time in both the first-person transform and
  the owner-backed `minecraft:crossbow/pull` item-model property; and
- preserve Vanilla stack-enchantment order and synchronized custom-definition
  precedence in every query.

Completion requires focused tests for value-effect parsing, random
advancement, local trident prediction/release, first-person crossbow
presentation, and owner-backed item-model resolution. The ledger TODO is
removed only after the client/server component partition has source-backed
reverse guards and the final engineering HEAD passes the full gate.

## Client/Server Partition

The synchronized definition crosses configuration, but execution ownership is
not implied by synchronization.

Client-observable components are:

- `ATTRIBUTES`, read by `EnchantmentHelper.forEachModifier`;
- `CROSSBOW_CHARGE_TIME`, read with `LivingEntity.getRandom()` by
  `CrossbowItem.getChargeDuration`;
- `TRIDENT_SPIN_ATTACK_STRENGTH`, read with `LivingEntity.getRandom()` by
  `TridentItem.use` and `releaseUsing`;
- `TRIDENT_SOUND`, selected before the `ServerLevel` throw branch; and
- `PREVENT_ARMOR_CHANGE`, read by client inventory/equipment interaction.

`CROSSBOW_CHARGING_SOUNDS` is server-owned despite being presentation data:
`CrossbowItem.onUseTick` enters its sound thresholds only under
`!level.isClientSide()`, and the resulting level sound is synchronized back to
clients.

Every conditional value family, `DAMAGE_IMMUNITY`, `POST_ATTACK`,
`POST_PIERCING_ATTACK`, `HIT_BLOCK`, `TICK`, `PROJECTILE_SPAWNED`, and
`LOCATION_CHANGED` executes through methods whose world parameter is
`ServerLevel`. The bbb client validates registry identities nested in those
synchronized definitions but does not execute their loot conditions or
gameplay effects.

## Non-Goals

- Do not implement server damage, durability, ammo, projectile, fishing, XP,
  loot-condition, entity-effect, or location-effect gameplay.
- Do not synthesize crossbow loading sounds locally.
- Do not add a generic Mojang Codec/DynamicOps or loot-table interpreter.
- Do not share mutable state between immutable world publications. Simulation
  and render projections keep independent client-local cursors, as they
  already do for item time/compass randoms and entity presentation randoms.
- Do not change wire formats, registry holder ordering, or resource-pack model
  selection APIs beyond the holder identity needed by the charge-time query.

## Frozen Ownership and APIs

- `bbb-world::registries::enchantments` remains the only parser and executor
  for synchronized `LevelBasedValue` and `EnchantmentValueEffect`.
- `LevelEventSoundRandomState` remains the repo's existing LegacyRandomSource
  implementation. The effect processor receives it explicitly and consumes
  `nextFloat`/`nextGaussian` in Vanilla order.
- The canonical local-player entity keeps one client-local random cursor for
  simulation-side trident use and release. The cursor is seeded
  deterministically from the protocol entity identity and reset with each
  incarnation because Vanilla's
  `RandomSource.create()` seed is process-random and not synchronized.
- `bbb-item-model::NativeItemRuntime` keeps one render-side cursor per
  `(entity_id, uuid)` owner. Resource reload may replace the runtime; within a
  runtime, all first-person transform and item-model charge queries for that
  owner advance the same cursor.
- Ownerless GUI, dropped-item, display-entity, and other non-living item
  consumers do not evaluate `CrossbowItem.getChargeDuration`; they retain the
  existing inactive/zero property behavior.
- `bbb-native` supplies the concrete living owner identity. It does not parse
  or execute enchantment effects independently.

## Vanilla 26.1 Evidence

`EnchantmentValueEffect.bootstrap` registers exactly `add`, `all_of`,
`multiply`, `remove_binomial`, `exponential`, and `set`.
`RemoveBinomial.process` uses one `nextFloat` per Bernoulli trial when
`n <= 128`, `n*p < 20`, or `n*(1-p) < 20`; otherwise it rounds
`floor(n*p) + nextGaussian()*sqrt(n*p*(1-p))` and clamps the removed count to
`0..(int)n`.

`EnchantmentHelper.modifyCrossbowChargingTime` and
`getTridentSpinAttackStrength` pass `holder.getRandom()`. `CrossbowPull.get`
returns zero without a living owner; with one, it calls
`CrossbowItem.getChargeDuration`. `ItemInHandRenderer` independently calls the
same duration for the first-person charging transform, so one rendered
first-person crossbow can advance the holder cursor once for the transform and
again for the item-model property.

The server-only partition is fixed by `Enchantment.java`: conditional value
helpers build loot contexts from `ServerLevel`; tick/projectile/post-attack
entity effects receive `ServerLevel`; and location activation runs only
through `runLocationChangedEffects(ServerLevel, ...)`.

All source facts come from
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft`.

## Ordered Stages and Focused Tests

### A. Value effects and simulation owner

Add `remove_binomial`, including the Marsaglia polar Gaussian cache, and make
random consumption explicit in the synchronized crossbow/trident APIs. Route
local trident preflight and release through the canonical local-player cursor.
Prove stack order, synchronized overrides, both algorithm branches, Gaussian
cache reuse, and repeated-query advancement.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world enchantment --lib
scripts/cargo-dev.sh test -p bbb-world trident --lib
```

### B. Render-side living-owner projection

Add the `(entity_id, uuid)` cursor map to `NativeItemRuntime`, reset it in
independent projections/reloads, and pass living owner identity through held
item baking. Use synchronized duration for the first-person transform and
owner-backed item-model use context, preserving Vanilla query order.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-item-model crossbow --lib
scripts/cargo-dev.sh test -p bbb-native first_person_crossbow
scripts/cargo-dev.sh test -p bbb-native crossbow_charge
```

### C. Boundary guards and closeout

Add structural tests that pin the five client component consumers and the
`ServerLevel`-owned remainder, remove the ledger TODO, archive this contract,
and append the engineering milestone index.

Focused tests:

```text
scripts/docs-lint.sh
scripts/cargo-dev.sh test -p bbb-world enchantment --lib
scripts/cargo-dev.sh test -p bbb-native crossbow
```

## Checkpoint, Failure, and Safe Stops

Stages A and B are separate clean, buildable commits. Run the full gate at the
final stage-C HEAD, after four accumulated commits, or immediately if focused
tests expose publication/cache isolation failures. A clean contract commit, a
world-only value-effect commit with existing deterministic callers preserved,
or a render cursor commit before ledger removal is a safe stop. Failures are
fixed forward; the ledger stays open until all stages pass.
