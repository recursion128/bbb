# Environment position sampling foundation

Status: completed on 2026-07-23.

## Goal and completion contract

Establish the missing Vanilla 26.1 positional layer of
`EnvironmentAttributeSystem` without prematurely adding new ambient-particle,
general music, ambient-sound, firefly-bush, or block-entity consumers. The
foundation must preserve each biome `EnvironmentAttributeMap` entry as a
modifier over the value produced by the dimension layer, then apply dimension,
biome, and timeline layers in that order. Existing client consumers that
already read a positional synchronized attribute must stop treating the
dimension/timeline sample as final.

Completion means a source-generated catalog owns every declared environment
attribute's resource id, attribute type, syncable, positional, and spatially
interpolated flags; biome typed projections retain every valid positional
modifier rather than reducing it against the attribute default; the world
exposes exact direct-noise-biome and camera-Gaussian sampling modes; modifiers
are applied before timeline tracks and sanitized afterwards; unknown,
malformed, or non-positional biome entries fail the registry candidate
atomically; malformed non-syncable entries fail before valid non-syncable
entries are filtered from the client map; synchronized holder reorder and
`ChunksBiomes` replacement immediately affect the next sample; and all
existing visual, sound, entity, fluid, and level-event consumers use the
correct positional or dimension-only query.

## Scope, ownership, and non-goals

- `scripts/` extracts `EnvironmentAttributes.java` through the shared
  Tree-sitter Java frontend and generates the checked-in descriptor catalog.
- `bbb-world` owns descriptor consumption, raw typed biome modifiers, direct
  and Gaussian biome sampling, dimension/biome/timeline ordering, sanitization,
  registry/session lifecycle, and focused semantic tests.
- `bbb-native` migrates existing camera render, selected background-music
  volume, piglin presentation, default-dripstone, and water-evaporation
  consumers to the new sample API. Camera attributes use Gaussian biome
  weights; entity, player, and block-position queries use the direct noise
  biome.

The frozen public integration surface is
`EnvironmentBiomeSampling::{Direct, Gaussian}` and
`WorldStore::sampled_client_environment_attributes_at(position: [f64; 3],
sampling) -> Result<ClientEnvironmentAttributes, TimelineProjectionError>`.
The existing zero-position
`sampled_client_environment_attributes()` remains the dimension/timeline
query for attributes whose Vanilla declarations are non-positional. Raw biome
modifier representation stays world-owned and is not exported to native or
renderer crates.

This slice does not add the missing 667-pair ambient-particle animate loop,
general delayed `MusicManager`, `BiomeAmbientSoundsHandler`, firefly-bush
random sound, or complex default-dripstone particle option providers. It does
not predict server-owned creaking-heart block/entity transitions. It does not
consolidate the already implemented native weather layers or add camera
tick-to-partial-tick probe history; those consumers receive the correctly
layered current value and retain their existing frame/tick timing. The
`CLIENTBOUND_CHUNKS_BIOMES` row remains partial until the new ambient consumers
observe biome replacement; this foundation must narrow, not overstate, that
remaining gap.

## Vanilla facts and fallback behavior

`EnvironmentAttribute.Builder` defaults to positional and not spatially
interpolated. Only `SKY_LIGHT_LEVEL` and `FAST_LAVA` call `notPositional`;
the RGB/ARGB, float, and angle declarations that call
`spatiallyInterpolated` use their attribute type's spatial lerp. The generator
derives those facts and the `AttributeTypes` reference from the declaration
chain rather than copying a Rust table.

`Biome.NETWORK_CODEC` uses `EnvironmentAttributeMap.NETWORK_CODEC`: decoding
validates every received entry and then filters non-syncable attributes.
Therefore malformed non-syncable values still reject the atomic registry
candidate, while valid non-syncable values are omitted rather than retained as
client positional state.

`EnvironmentAttributeSystem.addDefaultLayers` orders dimension constants,
biome positional layers, dimension timelines, then weather. A biome layer
applies its stored modifier to the incoming value. When a caller provides a
`SpatialAttributeInterpolator` and the attribute is spatially interpolated,
the modifier result from each distinct biome map is combined with the
attribute type's spatial lerp. Otherwise the layer uses
`BiomeManager.getNoiseBiomeAtPosition`, which floors each coordinate to quart
space without the block-biome zoom/fiddle algorithm.

`EnvironmentAttributeProbe.tick` builds its interpolator with
`GaussianSampler.sample(position.scale(0.25), getNoiseBiomeAtQuart, ...)`.
The existing bbb Gaussian kernel and quart lookup are retained only after
their weights, traversal order, duplicate-biome-map accumulation, color
sRGB interpolation, float interpolation, and angle `90`-degree cutoff are
verified against `GaussianSampler`, `SpatialAttributeInterpolator`,
`AttributeTypes`, and `LerpFunction`. Missing chunk/holder data makes the
positional layer a no-op over its input; it never substitutes a bootstrap
ordinal for a synchronized holder id.

Authoritative local sources are
`net/minecraft/world/attribute/EnvironmentAttribute.java`,
`EnvironmentAttributes.java`, `EnvironmentAttributeMap.java`,
`EnvironmentAttributeSystem.java`, `EnvironmentAttributeProbe.java`,
`SpatialAttributeInterpolator.java`, `GaussianSampler.java`,
`AttributeTypes.java`, `LerpFunction.java`,
`net/minecraft/world/level/biome/Biome.java`,
`BiomeManager.java`, and
`net/minecraft/client/multiplayer/ClientPacketListener.java#handleChunksBiomes`.

## Serial stages and verification

1. Add the Tree-sitter extractor, generator tests, generated descriptor
   catalog, and reverse guards. Refactor environment decoding to consume the
   catalog, then retain raw typed positional modifiers in
   `BiomeClientProjection`. Focused checks run the generator unit tests and
   `scripts/cargo-dev.sh test -p bbb-world environment_attribute_map`.
2. Add direct and Gaussian world sampling with exact layer ordering, holder
   reorder/replacement/session reset behavior, malformed-candidate atomicity,
   modifier/type coverage, duplicate-map weight accumulation, and
   missing-chunk fallback. Focused tests use
   `scripts/cargo-dev.sh test -p bbb-world environment_position`.
3. Migrate every existing positional consumer and remove the bespoke
   four-field camera-biome path once equivalent RGB/float tests pass. Run
   `scripts/cargo-dev.sh test -p bbb-native environment_position`, plus the
   affected fog, sky, lightmap, music, entity-scene, fluid, and level-event
   focused regressions.
4. Narrow the exact environment and biome TODOs, update generated checks and
   long-term milestone index, archive this contract, review the integrated
   diff, and run `scripts/cargo-dev.sh gate`,
   `scripts/playable-gate.sh live`, and `scripts/playable-gate.sh auto`.

The fully gated `1d507a142` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Generator/catalog failures stop before world representation;
world sampling failures stop before consumer migration. This contract plus
three implementation/closeout commits forces the full gate at the milestone
head. Failures are fixed forward without rewriting history.
