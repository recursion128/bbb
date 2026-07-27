# Synchronized Environment ParticleOptions Closure

Status: completed on 2026-07-27.

## Goal and completion contract

Close the synchronized environment-attribute `ParticleOptions` path from
registry NBT to the existing native particle command resolver. A selected
dimension or timeline must preserve the exact typed option payload used by
Vanilla for both ambient particles and level event `1504`; block, item, color,
power, scale, roll, trail, vibration, and delay values must reach the same
provider/submission fields as equivalent packet particle options.

The slice is complete when:

- the world registry owner decodes every non-empty Vanilla 26.1 particle
  MapCodec family into one typed canonical option value while retaining the
  bounded original NBT fields for diagnostics;
- registry-backed block/item/component references are validated against the
  frozen session generation, including the complete nested
  `ItemStackTemplate.CODEC -> DataComponentPatch.CODEC` shape;
- ambient environment particles no longer reject valid non-empty options, and
  their provider command carries the decoded option values;
- pointed-dripstone event `1504` carries the exact sampled environment particle
  object rather than reducing it to a particle type id, while real water/lava
  still select Vanilla's fixed dripping particles;
- malformed/unknown option payloads fail the registry projection without
  creating a partial typed value, and missing provider assets retain the
  existing diagnostics/fallback behavior; and
- focused world/native/renderer semantic tests, the final full gate, and the P0
  audit pass on a clean committed tip.

## Non-goals

- Packet `ParticleOptions.STREAM_CODEC` wire changes or new packet families.
- Completing every particle provider's motion/rendering nuance, terrain
  destroy-effect presentation, or the open all-provider parity family.
- Custom particle-type registry entries; Vanilla's particle-type registry is a
  fixed built-in registry in 26.1.
- Adding a second item-component codec or an environment-only item model
  resolver. The existing dialog/component codec and native item runtime remain
  the reusable owners.
- Changing environment sampling, timeline interpolation, particle capacity,
  limiter, atlas, resource-generation, or renderer submission ownership.

## Boundaries and integration API

- `bbb-world::registries::environment_attributes` owns the map-codec result:
  `ClientParticleOptions` retains its normalized resource id and bounded source
  fields and gains a typed option payload. The payload stores float/double
  values as exact bits where canonical equality/serialization requires it.
- `bbb-world::client::dialog::body` remains the single
  `ItemStackTemplate`/persistent `DataComponentPatch` codec. A narrow
  registry-aware decode API may return the already existing
  `ItemStackSummary`; environment attributes must delegate to it rather than
  introduce another component switch.
- Dimension and timeline validation both use their existing
  `FrozenRegistryAccess`. A particle option is accepted only after its block
  state, item, nested component holders, and other registry references validate
  against that same generation.
- `bbb-native::particle_runtime` converts the typed world payload to the
  existing `ParticleOptionRenderState` and then uses the existing
  `command_for_type`/sprite/tint/provider path. Packet raw-option decoding stays
  an input adapter to that same semantic state.
- `LevelEventParticleContext` carries the sampled `ClientParticleOptions`
  value. Water/lava are constructed as typed empty-option particles; there is
  no `Other(type_id)` lossy branch.
- No new renderer API is expected. Existing `ParticleSpawnCommand` option
  fields and `ParticleInstance` provider behavior are the integration surface.

## Vanilla 26.1 facts

- `AttributeTypes.PARTICLE` is
  `AttributeType.ofNotInterpolated(ParticleTypes.CODEC)`;
  `AMBIENT_PARTICLES` is a list whose element particle uses the same codec
  (`AttributeTypes.java`, `AmbientParticle.java`).
- `ParticleTypes.CODEC` dispatches by the built-in particle type and invokes
  that type's MapCodec (`ParticleTypes.java`). The non-empty families are:
  block state; optional power; dust; dust color transition; spell; ARGB color;
  sculk roll; item stack template; safe block-position vibration; trail; and
  shriek delay. All other registered types use the empty simple codec.
- `ItemParticleOption.codec` is
  `ItemStackTemplate.CODEC.fieldOf("item")`.
  `ItemStackTemplate` requires a non-air item, count `1..=99`, and an optional
  `DataComponentPatch.CODEC`; patch keys dispatch to each persistent component
  value codec (`ItemParticleOption.java`, `ItemStackTemplate.java`,
  `DataComponentPatch.java`).
- `VibrationParticleOption.CODEC` rejects entity position sources in map data;
  only its safe position-source codec is accepted. Its network stream codec is
  broader, so the environment decoder must not copy the packet rule.
- `EnvironmentAttributes.DEFAULT_DRIPSTONE_PARTICLE` defaults to
  `DRIPPING_DRIPSTONE_WATER`; the Nether dimension overrides it with
  `DRIPPING_DRIPSTONE_LAVA` (`EnvironmentAttributes.java`,
  `DimensionTypes.java`).
- `LevelEventHandler` event `1504` calls
  `PointedDripstoneBlock.spawnDripParticle`. `getDripParticle` returns the
  sampled environment attribute only for `Fluids.EMPTY`; actual lava selects
  `DRIPPING_DRIPSTONE_LAVA`, and every other non-empty fluid path selects
  `DRIPPING_DRIPSTONE_WATER`. The resulting complete `ParticleOptions` object
  is passed unchanged to `Level.addParticle`
  (`LevelEventHandler.java`, `PointedDripstoneBlock.java`).

## Serial stages and validation

1. **Typed registry codec.** Add the canonical option payload, delegate item
   templates/components to the existing registry-aware codec, validate
   dimension and timeline references, and cover every MapCodec family plus
   malformed/default/registry-reorder cases.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world environment_attributes` and
   `scripts/cargo-dev.sh test -p bbb-world dimension_type`.
2. **Native effects integration.** Convert typed environment options through
   the existing particle resolver, replace the lossy dripstone type enum, and
   prove exact ambient/default-dripstone option fields, item-model lookup,
   provider acceptance, counter behavior, and water/lava precedence.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native environment_particle` and
   `scripts/cargo-dev.sh test -p bbb-native dripstone_drip`.
   This is the cross-boundary checkpoint and ends with a full gate.
3. **Closeout.** Delete the completed ledger wording, archive this contract,
   append the engineering milestone, review the integrated diff, then run the
   final full gate and P0 audit.

Every stage is a buildable normal commit after focused tests,
`cargo fmt --all --check`, `git diff --check`, documentation lint when
applicable, and full diff review. Failures are fixed forward from the preceding
clean stage; history is never rewritten.

## Hardest-ready decision

The four remaining packet-ledger `partial` entries are all guarded by explicit
deferred presentation prerequisites, and the registry/item-component ledgers
have no other partial rows. Full survival physics, all-provider parity, broad
dynamic textures, and the remaining performance families are open-ended rather
than one freezeable semantic contract. This slice is their hardest bounded
ready peer: it crosses synchronized registry codecs, nested item components,
timeline/dimension generation identity, world-to-runtime effects, provider
options, item/terrain sprite selection, and level-event precedence, while
Vanilla fixes every codec field and execution branch above.
