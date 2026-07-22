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
item-stack hover-name override is also closed on that path: world reads the
displayed stack's exact `custom_name`, native applies render-names/crosshair/
distance eligibility plus the facing-dependent renderer offset, and the same
typed surfaces preserve ordered see-through and normal passes.

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

The following cross-boundary stage closes `AvatarRenderer`'s `ArrowLayer` and
`BeeStingerLayer`. `bbb-world` reads the generated LivingEntity
`DATA_ARROW_COUNT_ID` / `DATA_STINGER_COUNT_ID` INT accessors only for players
and preserves the synchronized values; `bbb-native` forwards the two counts
without sampling attachment points. `bbb-renderer` owns the deterministic
`StuckInBodyLayer` placement because it alone owns the posed `PlayerModel` cube
tree: seed the 48-bit `SingleThreadedRandomSource` with `EntityRenderState.id`,
select the vanilla body-part order `head, body, leftArm, rightArm, leftLeg,
rightLeg`, consume the cube selection, then sample three coordinates. Arrows
use `IN_CUBE`; bee stingers use `ON_SURFACE`, including the extra random plane
and `value > 0.5 ? 1.0 : 0.5` face snap. Both layers inherit the selected posed
part transform, use no overlay and the entity outline color, and retain
`AvatarRenderer` constructor order: arrow after item-in-hand and before extra
ears, stinger after the spin effect. The renderer caps each untrusted synced
count at 64 per player per frame; ordinary vanilla state is unchanged while a
hostile INT cannot create an unbounded submission loop.

This stage reuses the existing `ArrowModel`/normal arrow texture and adds the
16x16 `BeeStingerModel` crossed-plane layer and
`textures/entity/bee/bee_stinger.png`. It does not implement swim-arm or elytra
`speedValue` poses, which require continuous animation inputs rather than this
metadata-to-layer contract. Focused tests are the `player_stuck` tests in
`bbb-world`, `bbb-native`, and `bbb-renderer`, plus the existing player and
renderer-catalog tests affected by layer ordering. The entire projection and
consumer path lands in one clean checkpoint commit and runs the full gate. A
safe stop exists only before implementation or after that fully wired commit;
the world counts are never landed without the bounded renderer consumer.

The next cross-boundary stage closes `AvatarRenderer` / `HumanoidModel`'s
continuous fall-flying and swimming pose contract. `bbb-world` owns the
client-tick facts: the generated fall-flying flag, LivingEntity's exact
`isVisuallySwimming` pose fallback, `getSwimAmount(partialTick)`, current
velocity-derived `HumanoidRenderState.speedValue`, player fall-flying ticks,
and `ClientAvatarState.deltaMovementOnPreviousTick`. At render extraction it
also computes `AvatarRenderer.extractFlightData` from the interpolated view
vector and previous-to-current velocity. `bbb-native` forwards these typed
values without re-running tick or trigonometric state.

`bbb-renderer` inserts `AvatarRenderer.setupRotations` after the shared living
setup rotation and before the model flip: fall flying applies the squared-time
pitch blend and optional movement/look yaw, while swimming applies the
in-water/crawling pitch and visually-swimming translation. The shared
humanoid pose uses `speedValue` for the initial limb swing and applies the
fall-flying/swimming head pitch override. `PlayerModel` then preserves vanilla
26.1 ordering for arm poses: item arm poses, attack, crouch, non-spyglass arm
bob, the three-phase 26-tick swim stroke with SPEAR/attack-arm suppression,
and the swim leg kick. Existing elytra wing rotations remain sourced from the
already canonical `ElytraAnimationState`; this stage changes their owning body
pose, not the wing animation source.

This stage does not change item classification, handedness policy, mob-specific
held-item refinements, or add a second animation clock. Focused tests are the
`player_flight_swim` tests in `bbb-world`, `bbb-native`, and `bbb-renderer`,
plus existing player, zombie, skeleton, piglin, wings, and stuck-projectile
pose regressions affected by the shared humanoid inputs. The complete
tick-to-render path lands in one clean checkpoint commit and runs the full
gate. A safe stop exists before implementation or after that fully wired
commit; renderer flight transforms never land with defaulted world inputs.

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

This final stage is a high-risk generated-catalog change and proceeds in three
serial checkpoints from fully gated commit `e10be4ff5`:

1. **Reverse classification.** The world renderer catalog derives one exhaustive
   presentation owner for every fixed type from the 155 generated
   `EntityRenderers` registrations plus Vanilla's explicit Player/Mannequin
   avatar exception. Native model selection consumes that owner; catalog tests,
   generated source guards, and a native reverse test reject `NoRender`, raw
   `Humanoid`/`Quadruped`, or `Placeholder` results outside the source-proven
   noop and separately owned special passes.
   Renderer dispatch names the three raw fallback variants explicitly so a new
   model kind cannot enter a wildcard fallback silently. Focused tests are the
   renderer-catalog generator tests, native `entity_model_kind` reverse tests,
   and renderer entity-model dispatch tests.
2. **Audit failures.** Any type rejected by the reverse proof is fixed through
   its existing semantic owner before the matrix changes. The expected first
   correction is Painting's not-ready path: `PaintingRenderer` is always owned
   by the tiled painting submission, so missing registry/atlas data must retain
   diagnostics and suppress geometry rather than falling through to a raw box.
   Further family gaps use their concrete renderer/model/layer classes and
   focused suites; animation, passenger/leash, and event/audio work stays in its
   separate coverage cell.
3. **Coverage closeout.** Only after the reverse proof is green, update
   `renderer_audit`, remove the model/layer matrix TODO and stale fallback TODOs,
   regenerate `docs/entity-coverage-26.1.json`, and append the milestone index.
   The stage commits at mechanically reviewable checkpoints, runs the full gate
   after at most four commits, and finishes with both the full gate and
   `scripts/playable-gate.sh`. A safe stop is a clean, fully gated reverse-proof
   checkpoint with the matrix still partial; a half-regenerated matrix is not a
   safe stop.

Closeout evidence is now green: the generated registration catalog uniquely
partitions all 157 fixed types, the native reverse test rejects every raw model
fallback for production `EntityModel` owners, and the renderer source guard
keeps those fallbacks out of wildcard dispatch. Separately owned item, frame,
display, painting, lightning, and block-attachment passes retain explicit
`NoRender` ownership; world Component text also selects resource-pack fonts and
frame-seeded equal-advance obfuscation. The full and playable gates below remain
the delivery exit criterion for the closeout commit.

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
