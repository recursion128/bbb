# Persistent Holder-Reference Coverage Closure

This document freezes the engineering contract for the 38 remaining Vanilla
26.1 named persistent Holder factories in the `persistent-holder-reference`
family of `docs/registry-holder-consumer-coverage-26.1.json`. Work starts from
fully gated commit `e7aec5c76`.

## Goal and Completion Standard

Every one of the 38 `RegistryFixedCodec`, `Registry.holderByNameCodec`, or
reference-only `RegistryFileCodec` call sites must receive one of two
source-backed outcomes:

- `complete` when a bbb client consumer decodes, validates, retains, or
  reconstructs the named resource identity against the correct fixed or
  synchronized registry; or
- `not-applicable` when the codec is exclusively a server save, advancement,
  loot, world-generation, point-of-interest, or generic codec helper path.

The fixed inventory is partitioned before implementation:

- 25 client-observable rows: fixed DamageType, GameEvent, enchantment,
  jukebox-song, map-decoration, item, potion, mob-effect, attribute, debug-stick,
  villager, and synchronized animal-variant identities; and
- 13 server-only rows: four advancement predicates, the generic
  `RegistryCodecs` delegation, packed server attributes, POI records, beacon
  save data, disk chunk biome palettes, structure placement, and three loot
  paths.

Completion requires all 38 rows to leave `partial`. A persistent named Holder
is exactly one resource-location string: lists, tags, inline compounds, unknown
resources, and malformed identifiers must fail. Synchronized identities bind
through the frozen session revision; built-in identities use the generated
fixed registry only when that registry is not server synchronized. No caller
may reinterpret a resource through bootstrap ordinal position.

## Frozen Source-Path Partitions

The 25 complete rows are selected only by these source paths (VillagerData
contains two rows):

- `net/minecraft/world/damagesource/DamageType.java`
- `net/minecraft/world/effect/MobEffect.java`
- `net/minecraft/world/entity/ai/attributes/Attribute.java`
- `net/minecraft/world/entity/animal/chicken/ChickenSoundVariant.java`
- `net/minecraft/world/entity/animal/chicken/ChickenVariant.java`
- `net/minecraft/world/entity/animal/cow/CowSoundVariant.java`
- `net/minecraft/world/entity/animal/cow/CowVariant.java`
- `net/minecraft/world/entity/animal/feline/CatSoundVariant.java`
- `net/minecraft/world/entity/animal/feline/CatVariant.java`
- `net/minecraft/world/entity/animal/frog/FrogVariant.java`
- `net/minecraft/world/entity/animal/nautilus/ZombieNautilusVariant.java`
- `net/minecraft/world/entity/animal/pig/PigSoundVariant.java`
- `net/minecraft/world/entity/animal/pig/PigVariant.java`
- `net/minecraft/world/entity/animal/wolf/WolfSoundVariant.java`
- `net/minecraft/world/entity/animal/wolf/WolfVariant.java`
- `net/minecraft/world/entity/npc/villager/VillagerData.java`
- `net/minecraft/world/entity/npc/villager/VillagerType.java`
- `net/minecraft/world/item/Item.java`
- `net/minecraft/world/item/JukeboxSong.java`
- `net/minecraft/world/item/alchemy/Potion.java`
- `net/minecraft/world/item/component/DebugStickState.java`
- `net/minecraft/world/item/enchantment/Enchantment.java`
- `net/minecraft/world/level/gameevent/GameEvent.java`
- `net/minecraft/world/level/saveddata/maps/MapDecorationType.java`

The 13 not-applicable rows are selected only by these source paths:

- `net/minecraft/advancements/criterion/BeeNestDestroyedTrigger.java`
- `net/minecraft/advancements/criterion/EnterBlockTrigger.java`
- `net/minecraft/advancements/criterion/PlayerPredicate.java`
- `net/minecraft/advancements/criterion/SlideDownBlockTrigger.java`
- `net/minecraft/core/RegistryCodecs.java`
- `net/minecraft/world/entity/ai/attributes/AttributeInstance.java`
- `net/minecraft/world/entity/ai/village/poi/PoiRecord.java`
- `net/minecraft/world/level/block/entity/BeaconBlockEntity.java`
- `net/minecraft/world/level/chunk/PalettedContainerFactory.java`
- `net/minecraft/world/level/levelgen/structure/placement/StructurePlacement.java`
- `net/minecraft/world/level/storage/loot/functions/CopyBlockState.java`
- `net/minecraft/world/level/storage/loot/functions/SetContainerLootTable.java`
- `net/minecraft/world/level/storage/loot/predicates/LootItemBlockStatePropertyCondition.java`

## Non-Goals

- The remaining persistent Holder-or-inline, HolderSet-or-inline, TagKey,
  hashed-tag, and custom HolderSet families stay separate ledger work.
- This slice does not reproduce Mojang's generic Codec/DynamicOps framework.
- bbb does not execute server advancement, loot, POI, world-generation, beacon
  save/load, or enchantment entity-effect gameplay merely because it validates
  a synchronized definition.
- Network Holder ids remain owned by their already closed network families;
  this slice owns the resource-name form used by persistent codecs.
- Existing context-free built-in component decoding remains legal only for a
  generated fixed registry identity with an exact resource-name lookup.

## Owners and Frozen Boundaries

- `bbb-world::registries` owns one persistent named-Holder NBT resolver. It
  accepts only a resource string, normalizes it, and returns a checked
  `FrozenRegistryEntry` from one `FrozenRegistryAccess`.
- Dialog component decoding and the synchronized timeline/game-test consumers
  reuse the registry resolver wherever they decode the same fixed named form;
  field-local semantic errors remain owned by those consumers.
- The synchronized enchantment materializer recursively validates
  `DamageType.CODEC` and `GameEvent.CODEC` references in exact polymorphic
  effect variants. Candidate failure remains atomic; validation does not imply
  effect execution.
- `bbb-native::PersistentSnbtContext::registry_resource_id` remains the
  persistent reconstruction owner for numeric network holders. It resolves a
  synchronized holder through the current session and uses generated fixed
  fallback only when no server contents exist.
- Existing typed component and entity-variant tests are evidence for the other
  client rows; no duplicate decoder is added solely to satisfy the ledger.
- Coverage families split only by the exact source-path partitions above. The
  server-only family has no bbb consumer.

## Vanilla 26.1 Evidence

`RegistryFixedCodec` encodes and decodes only an `Identifier` through the
matching `RegistryOps` owner/getter and rejects direct values. A registry's
`holderByNameCodec` likewise narrows to reference holders. The one included
`RegistryFileCodec` call explicitly passes `allowInline = false` for a server
structure-set exclusion reference.

`RegistryDataLoader.SYNCHRONIZED_REGISTRIES` includes DamageType,
Enchantment, JukeboxSong, every listed animal variant/sound variant, and the
other existing dynamic client registries. `DataComponents.DAMAGE_TYPE`,
enchantment maps, jukebox-playable, named variant components, map decorations,
and other component codecs preserve named identities on disk while their
network codecs use holder ids.

Within synchronized `Enchantment.DIRECT_CODEC`, `DamageEntity.damage_type` is
required, `ExplodeEffect.damage_type` is optional, and `ReplaceBlock`,
`ReplaceDisk`, and `SetBlockProperties` carry optional
`trigger_game_event`. Their type discriminators are registered by
`EnchantmentEntityEffect` and `EnchantmentLocationBasedEffect`.

## Ordered Stages and Focused Tests

### A. Registry-owned persistent named-Holder resolver

Add the exact string-only resolver and typed errors to frozen registry access.
Reuse it from the persistent HolderSet scalar branch and migrate matching
dialog, timeline, and game-test reference decoders without changing their
published semantic shapes.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world persistent_holder
scripts/cargo-dev.sh test -p bbb-world timeline --lib
scripts/cargo-dev.sh test -p bbb-world game_test --lib
```

### B. Synchronized reference consumers

Validate nested DamageType and GameEvent effect references during enchantment
candidate materialization. Extend existing component/variant persistence tests
to prove custom synchronized order, fixed fallback, malformed input, missing
resource, configuration re-entry, and atomic failure.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world enchantment --lib
scripts/cargo-dev.sh test -p bbb-native holder
```

### C. Coverage classification and closeout

Split the broad family into the two exact source-path partitions, regenerate
the ledger, and assert 25 complete rows, 13 not-applicable rows, and zero
partial/deferred rows within this inventory. Reverse tests pin the shared
resolver, synchronized effect validation, persistent reconstruction, and the
absence of a consumer for server-only rows.

Focused tests:

```text
scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check
scripts/cargo-dev.sh test -p bbb-protocol registry_holder_coverage
```

## Checkpoints, Failures, and Safe Stops

The contract, shared resolver, synchronized consumers, and coverage closeout
are separate buildable commits. Run a full gate by the fourth commit, at the
final engineering HEAD, and before switching semantic domains. A focused
candidate-materialization failure triggers a full gate after the fix.

Safe stops are a clean contract commit; a shared resolver with migrated callers
and unchanged valid projections; atomic synchronized validation without ledger
status changes; or a source-backed blocker documented against the exact row.
Failures are fixed forward, and no coverage row changes status before its
consumer tests or server-only proof lands.
