# Entity Model, Texture, and Layer Closure

This document freezes the engineering contract for closing the Vanilla 26.1
`model/texture/layers` coverage cell for all 157 fixed entity types. Work starts
from full-gated commit `17d3eb8b1`.

## Goal and Completion Standard

Every fixed entity type must have a source-backed rendering disposition:

- an official dedicated or shared model submission with its Vanilla texture,
  variant, scale, attached layers, light, shadow, name, and culling behavior;
- a source-proven `NoopRenderer` or an owned non-model pass; or
- a narrow documented fallback that keeps the matrix cell partial.

The family closes only after reverse-checking every generated
`EntityRenderers` registration against the production dispatch, removing every
official non-noop entity from the raw placeholder path, updating the coverage
matrix and unsupported-feature ledger, and passing focused tests, the full
gate, and `scripts/playable-gate.sh` at current HEAD.

The authoritative flow is:

```text
generated renderer registration + synchronized world state
  -> narrow WorldStore render sources and exact visibility rules
  -> native asset/model resolution and per-frame transforms
  -> renderer-owned typed submission surfaces
  -> catalog reverse tests and 157-row coverage evidence
```

## Non-Goals

- Transformation/keyframe interpolation, procedural pose animation, and
  display interpolation belong to the `animation` coverage cell. This slice
  may submit the current synchronized pose without marking animation complete.
- Passenger placement, equipment semantics, and leash ownership remain in the
  `equipment/passenger/leash` cell, except renderer-owned layers that merely
  consume an already projected attachment.
- Entity events, particles, and audio remain in their own cell.
- Do not implement server AI, manufacture server-private state, add an in-game
  configuration UI, or replace missing resource-pack assets with permanent raw
  cuboids.

## Owners and Frozen Boundaries

- `bbb-protocol` owns decoded metadata value shapes and fixed entity/item ids;
  no wire changes are planned.
- `bbb-world` remains the canonical owner of synchronized metadata, transforms,
  variants, equipment, and light sampling. It exposes narrow immutable render
  sources and visibility facts; it does not bake meshes or retain camera state.
- `bbb-native` resolves current world sources against item, block, texture, and
  registry runtimes, computes camera-dependent transforms, and merges outputs
  into the existing single-frame renderer commit.
- `bbb-renderer` owns typed GPU submission surfaces, render phases, and resource
  residency. It does not query live world state or infer entity families.
- `scripts/data/entity-coverage-overrides-26.1.json` remains the classification
  input. `crates/bbb-world/src/entities/renderer_catalog/generated_26_1.rs`
  remains the exhaustive renderer-registration inventory.

New special paths must join an existing semantic owner or introduce one narrow
typed surface. They must not add a second entity scene, a camera cache in world,
or a texture-backed direct-mesh bypass.

## Vanilla 26.1 Evidence

The main agent rechecks local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each stage. Initial evidence is:

- `EntityRenderers.java` defines the exhaustive fixed registrations and the
  renderer class used by each type.
- `EntityRenderer.java` defines render-distance gating, the culling box inflated
  by `0.5`, the zero/NaN four-block fallback, leash-holder visibility, light,
  outline, name, fire, and shadow extraction.
- `Display.java` owns metadata ids 8 through 27, zero-size no-frustum behavior,
  the width/height culling box, `view_range * 64 * Entity.getViewScale()` distance,
  brightness/glow overrides, and transformation/substate interpolation.
- `DisplayRenderer.java` applies billboard orientation before translation,
  left rotation, scale, and right rotation. Block display submits a block model;
  item display rotates Y by pi and submits the selected item display context;
  text display rotates Y by pi, scales by `-0.025`, lays out ten-pixel lines,
  then submits optional background and styled text in ordered phases.
- The relevant concrete renderer and model/layer classes remain authoritative
  for each subsequent catalog family. Memory or historical ledger wording is
  not accepted when a local source file exists.

## Ordered Stages and Tests

### A. Display trio and special-path inventory

Project current Display metadata through a narrow world source; implement exact
view-range and width/height culling; add dedicated block, item, and text
submissions with official transforms, light, flags, and resource fallbacks.
Replace the three placeholder dispatches with the owned paths. Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world display
scripts/cargo-dev.sh test -p bbb-native display
scripts/cargo-dev.sh test -p bbb-renderer display
```

### B. Registration-by-registration model and layer closure

Reverse-check every generated renderer class against production dispatch.
Close missing renderer-owned model variants, textures, attached layers, shadow,
name, outline, fire, and exceptional culling in coherent source families.
Each stage names its concrete Vanilla renderer/model classes and runs the
affected crate's family-focused tests plus the renderer-catalog reverse tests.

The first cross-cutting stage is closed for generic entity name displays from
`EntityRenderer`, `LivingEntityRenderer`, `MobRenderer`, and
`ArmorStandRenderer`: generate the `EntityType.Builder.nameTagOffset` override,
project synchronized custom/player names, team visibility and below-name score,
resolve camera/crosshair-dependent eligibility in native, and submit the
ordered see-through plus normal font passes atomically. `ItemFrameRenderer`'s
item-stack hover name remains with the item-frame family because it reads the
displayed stack rather than the entity name.

Its bounded protocol/world prerequisite replaces the old trailing-byte
`NumberFormat` summary with the source-generated `NumberFormatTypes.bootstrap`
ids and typed blank/styled/fixed payloads. `SetObjective`, `SetScore`, and
`SetPlayerTeam` retain their trusted rich Components as exact validated NBT,
plain compatibility fields, and inherited styled runs in canonical scoreboard
state;
`Entity.belowNameDisplay` formatting therefore stays a world-owned query rather
than being reconstructed from lossy strings in native or renderer code.

The next cross-cutting stage closes the dispatcher-owned fire and shadow
features in two ordered commits:

1. **Fire.** `bbb-world` projects `Entity.displayFireAnimation()` from the
   synchronized on-fire flag, generated `EntityType.fireImmune` property, and
   exact current AABB width/height on the already-culled model source. `bbb-native` resolves
   `minecraft:block/fire_0` and `fire_1` through the resident blocks atlas and
   bakes the camera-Y-only layered quads. `bbb-renderer` owns one typed flame
   surface and draws it in the solid feature phase after entity model/model-part
   submissions and before leash/item/block/custom submissions. Spectators and
   hidden first-person camera entities remain excluded by the existing
   visibility generation; missing atlas sprites follow the blocks-atlas fallback
   sprite contract rather than suppressing the feature.
2. **Shadow.** The generated renderer registration catalog owns each fixed
   renderer's base `shadowRadius` and `shadowStrength`; mob age scaling and
   Display metadata remain explicit world projection rules. `bbb-world` applies
   the options/invisibility/distance gates, samples the exact bounded supporting
   blocks and local raw brightness, and publishes immutable shadow pieces.
   `bbb-native` resolves the dispatcher-root offset rule (including Avatar's
   offset cancellation), while `bbb-renderer` owns the shadow texture, blended
   quad surface, and first position in the translucent entity feature phase.

The fire commit runs focused `bbb-world`, `bbb-native`, and `bbb-renderer`
entity-feature tests and the renderer-order source guards. The shadow commit
adds the renderer-catalog generator checks plus focused block/light projection
tests. Because both stages cross world/native/renderer and alter feature phase
ordering, each is a clean checkpoint and runs the full gate before the next
stage starts. The safe stop is a committed, fully gated fire feature with the
old no-shadow behavior intact; shadow extraction is never landed without its
texture/pipeline consumer in the same commit.

The next coherent renderer-family stage closes `HappyGhastRenderer`'s
`RopesLayer`. `bbb-world` projects the generated `IS_LEASH_HOLDER` Boolean only
for happy ghasts and retains the existing pack-derived harness classification;
`bbb-native` forwards that narrow fact without inspecting metadata; and
`bbb-renderer` submits the adult or baby `HappyGhastModel` baked from
`createBodyLayer(..., CubeDeformation(0.2F))` with
`happy_ghast_ropes.png`. The layer renders only when both the synchronized
leash-holder flag and synchronized `minecraft:harnesses` item-tag membership
are present, reuses the base model's
tentacle animation and body-item squeeze, and follows `SimpleEquipmentLayer` in
constructor order. Player stuck-projectile layers and procedural arm/elytra
poses are non-goals for this stage because they require a separate random
attachment and animation-state contract. Focused tests are the `happy_ghast`
tests in `bbb-world`, `bbb-native`, and `bbb-renderer`; the complete path lands
in one clean commit and runs the full gate before another renderer family.

### C. Matrix closure and delivery gate

Prove that every official non-noop registration reaches a dedicated/shared
model or typed non-model submission, that no production type uses raw
placeholder geometry, and that all resource fallbacks are diagnosed and
bounded. Regenerate/check the matrix, remove completed TODOs, record the closed
engineering milestone in `docs/goal-archive.md`, then run:

```text
scripts/cargo-dev.sh gate
scripts/playable-gate.sh
```

## Checkpoints, Failures, and Safe Stops

The baseline was fully gated before work began. Run a full gate at every
cross-boundary milestone, after at most four commits, before changing semantic
families, and before stopping or delivery. A focused failure is fixed forward;
`batch-gate` is used only when the introducing commit is unclear.

Safe stops are limited to a clean, tested stage boundary: a world-only source
contract that has no runtime consumer yet; a complete typed native/renderer
path for one coherent renderer family; or a documented blocker with the old
fallback still intact. Never stop after removing a fallback but before its new
submission is connected to the atomic renderer frame.
