# Persistent Holder-or-Inline Coverage Closure

This document freezes the engineering contract for the 23 remaining Vanilla
26.1 `RegistryFileCodec` rows in the `persistent-holder-or-inline` family and
the one remaining `network-holder-or-inline` Instrument row of
`docs/registry-holder-consumer-coverage-26.1.json`. Work starts from fully gated
commit `b7648076a`.

## Goal and Completion Standard

Every one of the 24 rows must leave `partial` with one source-backed outcome:

- `complete` when a bbb client consumer preserves the named-versus-direct
  holder shape, resolves a named resource through the correct frozen registry,
  and validates the client-consumed direct value; or
- `not-applicable` when the factory is exclusively a generic delegation,
  server world-generation, level-stem, trial-spawner persistence, or loot
  loading path.

The fixed persistent inventory is partitioned before implementation:

- three client-observable rows: nested synchronized test environments,
  SoundEvent holders embedded in synchronized registries and item components,
  and Instrument item components; and
- twenty non-client rows: one generic HolderSet helper plus nineteen
  level/world-generation, trial-spawner, and loot factories.

The one network row is `Instrument.STREAM_CODEC`. A direct network Instrument
must retain its SoundEvent holder, positive duration and range floats, and exact
description Component NBT so persistent reconstruction can emit the equivalent
direct value. A named holder must retain only the current-session holder id and
must reconstruct through that session's resource identity. Unknown ids,
malformed direct payloads, and missing exact Component trees fail closed; no
bootstrap ordinal may substitute for a synchronized holder.

## Frozen Source-Path Partitions

The three persistent `complete` rows are selected only by these source paths:

- `net/minecraft/gametest/framework/TestEnvironmentDefinition.java`
- `net/minecraft/sounds/SoundEvent.java`
- `net/minecraft/world/item/Instrument.java`

The twenty persistent `not-applicable` rows are selected only by these source
paths:

- `net/minecraft/core/RegistryCodecs.java`
- `net/minecraft/world/level/biome/Biome.java`
- `net/minecraft/world/level/biome/MultiNoiseBiomeSourceParameterList.java`
- `net/minecraft/world/level/block/entity/trialspawner/TrialSpawnerConfig.java`
- `net/minecraft/world/level/dimension/DimensionType.java`
- `net/minecraft/world/level/levelgen/DensityFunction.java`
- `net/minecraft/world/level/levelgen/NoiseGeneratorSettings.java`
- `net/minecraft/world/level/levelgen/carver/ConfiguredWorldCarver.java`
- `net/minecraft/world/level/levelgen/feature/ConfiguredFeature.java`
- `net/minecraft/world/level/levelgen/flat/FlatLevelGeneratorPreset.java`
- `net/minecraft/world/level/levelgen/placement/PlacedFeature.java`
- `net/minecraft/world/level/levelgen/presets/WorldPreset.java`
- `net/minecraft/world/level/levelgen/structure/Structure.java`
- `net/minecraft/world/level/levelgen/structure/StructureSet.java`
- `net/minecraft/world/level/levelgen/structure/pools/StructureTemplatePool.java`
- `net/minecraft/world/level/levelgen/structure/templatesystem/StructureProcessorType.java`
- `net/minecraft/world/level/levelgen/synth/NormalNoise.java`
- `net/minecraft/world/level/storage/loot/LootTable.java`
- `net/minecraft/world/level/storage/loot/functions/LootItemFunctions.java`
- `net/minecraft/world/level/storage/loot/predicates/LootItemCondition.java`

The network `complete` row is selected by the Instrument source path and
`minecraft:instrument` registry identity already isolated by the broad family.

## Non-Goals

- The remaining TagKey, hashed-tag, custom HolderSet, and HolderSet-or-inline
  families stay separate ledger work.
- This slice does not reproduce Mojang's generic Codec or DynamicOps framework.
- bbb does not execute GameTest, world-generation, trial-spawner server logic,
  loot tables, item modifiers, or loot predicates.
- `Biome.CODEC` and `DimensionType.CODEC` are level-generation holder codecs;
  synchronized client registry elements use their separate `NETWORK_CODEC`
  values and remain owned by the existing typed projections.
- SoundEvent validation does not imply implementing every enclosing
  enchantment effect or item behavior. It only prevents synchronized or
  persistent client data from retaining an invalid holder identity.

## Owners and Frozen Boundaries

- `bbb-protocol::packets::data_components` owns the exact Instrument stream
  shape. Its direct summary keeps a typed SoundEvent, raw float bits, styled
  description, and exact NBT Component; existing tooltip summary fields remain
  derived compatibility projections.
- `bbb-world::registries` owns named-versus-direct persistent holder routing.
  Named values use `FrozenRegistryAccess::persistent_holder_from_nbt`; direct
  values remain borrowed NBT for the enclosing typed decoder to validate.
- Test-environment decoding, dialog item-component decoding, synchronized
  Instrument/JukeboxSong/animal-sound/Biome projections, and synchronized
  enchantment validation reuse that registry boundary without sharing their
  distinct presentation types.
- `bbb-native::PersistentSnbtContext` owns network-to-persistent Instrument
  reconstruction. Named ids resolve against the current session; direct values
  require all retained fields and emit the exact description Component.
- Coverage families split only by the exact source-path partitions above. The
  server-only family has no bbb consumer.

## Vanilla 26.1 Evidence

`RegistryFileCodec.decode` first attempts `Identifier.CODEC`; with RegistryOps,
a successful identifier resolves through the matching getter, while a failed
identifier decode falls through to the direct element codec when inline values
are allowed. `RegistryFileCodec.encode` likewise emits a resource identifier
for a reference holder and the direct element codec for an inline holder.

`RegistryDataLoader.SYNCHRONIZED_REGISTRIES` sends Instrument and TestEnvironment
with `DIRECT_CODEC`. A TestEnvironment `all_of` value recursively uses
`TestEnvironmentDefinition.CODEC`, so its nested holders are client-observable.
`DataComponents.INSTRUMENT` uses `Instrument.CODEC` persistently and
`Instrument.STREAM_CODEC` on the wire. `Instrument.DIRECT_CODEC` fields are, in
order, `sound_event`, `use_duration`, `range`, and `description`; its stream
codec preserves the same order.

SoundEvent is a client built-in registry. `SoundEvent.CODEC` accepts either its
named fixed identity or the direct `{sound_id, range?}` value and is embedded in
Biome environment attributes, synchronized animal sound variants, Instrument,
JukeboxSong, and Enchantment effect data. Named values must exist in the fixed
SoundEvent registry even when their enclosing registry is session-synchronized.

The other nineteen concrete persistent factories are reached only from
LevelStem/chunk-generator/worldgen registries, TrialSpawner server storage, or
loot data. Registry synchronization serializes each listed client registry's
element codec, not those holder codecs; the generic `RegistryCodecs` row merely
delegates the separately inventoried HolderSet factories.

## Ordered Stages and Focused Tests

### A. Preserve direct Instrument network values

Extend the protocol summary and native persistent encoder without changing the
named holder path or tooltip projection. Prove exact field order, named/direct
exclusivity, direct SoundEvent variants, float bits, exact Component NBT,
session reorder, and fail-closed incomplete reconstruction.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-protocol instrument --lib
scripts/cargo-dev.sh test -p bbb-native instrument_encoder --lib
scripts/cargo-dev.sh test -p bbb-item-model instrument --lib
```

### B. Close persistent client consumers

Add registry-owned named/direct routing and migrate nested test environments
and persistent dialog holders. Validate named SoundEvent references in every
client-consumed synchronized registry path, including Instrument, JukeboxSong,
animal sound variants, Biome environment attributes, and recursive Enchantment
sound effects. Registry candidate failure remains atomic.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world persistent_holder --lib
scripts/cargo-dev.sh test -p bbb-world game_test --lib
scripts/cargo-dev.sh test -p bbb-world instrument --lib
scripts/cargo-dev.sh test -p bbb-world jukebox --lib
scripts/cargo-dev.sh test -p bbb-world sound_variant --lib
scripts/cargo-dev.sh test -p bbb-world environment --lib
scripts/cargo-dev.sh test -p bbb-world enchantment --lib
```

### C. Coverage classification and closeout

Split the persistent family into the exact three-row complete and twenty-row
not-applicable partitions, mark the isolated network Instrument family complete,
regenerate the ledger, and add reverse coverage assertions for protocol
retention, registry routing, synchronized validation, native reconstruction,
and the absence of consumers for server-only rows.

Focused tests:

```text
scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check
scripts/cargo-dev.sh test -p bbb-protocol registry_holder_coverage --lib
```

## Checkpoints, Failures, and Safe Stops

The contract, protocol/native retention, registry validation, and coverage
closeout are separate buildable commits. The pre-start high-risk gate is
`b7648076a`. Run a full gate at the fourth commit and at the final engineering
HEAD; run the playable gate before milestone archive. Any protocol decode
failure outside the focused family or any atomic registry rollback failure
triggers an immediate full gate after the fix.

Safe stops are a clean contract commit; complete Instrument retention with no
ledger status change; complete registry validation with unchanged coverage; or
an exact source-backed blocker recorded against one row. Failures are fixed
forward, and no coverage row changes status before its consumer tests or
server-only proof lands.
