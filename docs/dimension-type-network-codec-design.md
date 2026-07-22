# DimensionType network-codec completion

## Goal and completion contract

Complete the client-owned projection of every field in vanilla 26.1
`DimensionType.NETWORK_CODEC`. A successful configuration revision must retain
the exact scalar values, the block `infiniburn` tag identity, the complete
`IntProviders.codec(0, 15)` monster-light provider, and the block-light limit in
the same frozen typed generation as the existing dimension shape, lighting,
environment attributes, timelines, and clock. Missing required fields,
malformed values, invalid ranges, and unknown provider types must reject the
candidate atomically and leave the prior revision visible. A syntactically
valid block tag remains a `TagKey` even when the current tag set has no such
entry, matching vanilla's non-resolving hashed codec.

The active level projects the client-observable values needed by existing
level behavior and diagnostics from the selected session holder. Completion is
proved with custom registry/tag order, all provider variants, malformed
candidate rollback, Login/Respawn holder selection, and focused non-test
checks.

## Scope and boundaries

- `bbb-world` owns typed NBT validation, frozen registry resolution, and the
  active-level projection.
- The existing configuration RegistryData decoder remains the wire owner; this
  slice does not add a second NBT or registry implementation.
- The existing `FrozenRegistryAccess` is the only tag-resolution boundary.
- `WorldLevelInfo` remains the serializable active-level/query projection.

This slice does not implement server world generation, monster spawning,
Ender Dragon fight setup, cross-dimension teleportation, or fire simulation.
It does not add built-in fallbacks for fields without a concrete current
consumer, and it does not change Biome, SlotDisplay, Ingredient, or worldgen
holder-set codecs.

## Vanilla and wire evidence

- `DimensionType.createDirectCodec` constructs both `DIRECT_CODEC` and
  `NETWORK_CODEC`; the network form differs only in the environment-attribute
  codec.
- `TagKey.hashedCodec(Registries.BLOCK)` encodes `infiniburn` as a required
  `#<identifier>` string.
- `MonsterSettings.CODEC` requires
  `IntProviders.codec(0, 15)` and an integer block-light limit in `0..=15`.
- `IntProviders.CODEC` accepts an integer constant or one of the seven built-in
  provider map codecs: constant, uniform, biased-to-bottom, clamped,
  weighted-list, clamped-normal, and trapezoid. The outer range validator uses
  each provider's derived minimum and maximum.
- `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` publishes dimension types during
  configuration; Login and Respawn select a `DimensionType.STREAM_CODEC`
  reference in current session order.

Authoritative sources are the corresponding files under
`~/Work/mc-code/sources/26.1/net/minecraft/`, especially
`world/level/dimension/DimensionType.java`, `tags/TagKey.java`, and
`util/valueproviders/*.java`.

## Serial stages and verification

1. Extend the frozen dimension projection with required scalar and hashed-tag
   validation. Bind tag queries to the same frozen block-tag state, expose
   applicable active-level values, and prove custom order plus rollback.
2. Add the complete bounded integer-provider value model and decoder, then
   retain the monster settings in the same projection. Prove every provider
   shape, nested/weighted derived bounds, range failures, and rollback.
3. Close the hashed-tag coverage row and unsupported-feature TODO, update the
   milestone index, review the integrated diff, and run the full gate.

Focused validation for each implementation stage is
`scripts/cargo-dev.sh test -p bbb-world registries` plus the named dimension
type and level tests, followed by `scripts/cargo-dev.sh check -p bbb-world`.
Every stage also runs formatting, diff checks, and docs lint when applicable.
The already fully gated `8dede8b67` is the pre-start checkpoint; the final
milestone HEAD receives `scripts/cargo-dev.sh gate`.

Each stage is a safe stopping point: it builds independently and candidate
validation remains atomic. Failures are fixed forward in the owning decoder or
projection; no prior commit or published registry generation is rewritten.
