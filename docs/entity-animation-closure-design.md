# Entity Animation Closure

This document freezes the engineering contract for closing the Vanilla 26.1
`animation` coverage cell for all 157 fixed entity types. Work starts from
full-gated commit `794e6ea41`.

## Goal and Completion Standard

Every fixed type must have one source-backed animation disposition:

- its current client-observable animation clocks and interpolation live in
  `bbb-world`, its render-state projection lives in `bbb-native`, and its exact
  keyframe/procedural pose is applied by its renderer-owned model or typed pass;
- a source-proven static model or non-model presentation explicitly performs no
  per-frame pose animation; or
- a narrow `not-applicable` decision explains why the official behavior depends
  only on server-private AI/RNG and cannot be reconstructed from multiplayer
  client state.

The category closes only after reverse-checking every production
`EntityModelKind` and separately owned presentation pass, removing the remaining
known pose gaps, regenerating the 157-row matrix, and passing focused tests, the
full gate, and `scripts/playable-gate.sh` at current HEAD.

## Non-Goals

- Equipment applicability, passenger placement/control, and leash ownership
  remain in `equipment/passenger/leash` unless a posed attachment merely follows
  an already-owned model part.
- Entity events, particles, and audio remain in their own coverage cell. An
  event may start a client animation clock here without moving its audiovisual
  side effects into this slice.
- Server AI, pathfinding, hit resolution, damage, spawning, and unobservable
  random animation start times are not synthesized.
- Display transform interpolation and remote movement interpolation stay with
  their already-closed transform/tick owners.

## Owners and Frozen Boundaries

- `bbb-world` owns tick-to-tick animation state, previous/current endpoints,
  synchronized metadata inputs, and event-started client clocks. It exposes
  typed immutable render-source fields and never bakes a model pose.
- `bbb-native` converts the current source into `EntityModelInstance` fields at
  partial tick. It does not re-run tick state machines or sample keyframes.
- `bbb-renderer` owns model trees, bind-pose reset, keyframe sampling,
  procedural `setupAnim` ordering, posed attachment transforms, and semantic
  geometry tests.
- The existing generated renderer catalog and production `EntityModelKind`
  selection remain the exhaustive fixed-type inventory. This engineering work
  does not add a second hand-maintained entity-type table or a broad public
  animation bag.
- Family changes stay in the existing semantic modules. A new clock or field is
  added only when a concrete Vanilla render state consumes it.

## Vanilla 26.1 Evidence

The main agent rechecks local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each family correction. Primary
anchors are `EntityRenderers`, each registered renderer's `extractRenderState`,
its model constructor, the model's `setupAnim`, referenced animation-definition
classes, and the entity's client `tick` / `handleEntityEvent` implementation.

Initial confirmed gaps are:

- `ArmadilloModel` selects distinct adult/baby walk, roll-out, roll-up, and peek
  definitions. The roll definitions animate the visible shell `cube`; the
  current renderer intentionally omitted some of those channels and reuses
  adult roll definitions for babies.
- `BabyAxolotlModel` is entirely keyframe-driven. It applies seven definitions
  selected by walk, swim, underwater/on-ground idle, and play-dead animation
  states; the current baby path stays at bind pose.
- `Creaking` has a client-tick tearing-down eye-flicker state that is consumed by
  presentation and must be classified with the exact model/layer owner.
- The reverse production inventory additionally found `GuardianModel` eye tracking: the current
  camera entity is the ordinary target, a live synchronized attack target takes priority, and the
  model converts the three world-space `GuardianRenderState` vectors into eye-part offsets.
- The shared `HumanoidModel.setupAnim` order separates limb swing from its
  trailing arm bob. Skeleton, piglin, and zombie-family paths must preserve
  pose/attack dispatch before that bob, including explicit `EMPTY` off-arm
  `yRot` reset and the subclass bob passes that intentionally run afterwards.
- `EnderDragonRenderer` derives landing/takeoff head droop from the exact
  `MOTION_BLOCKING_NO_LEAVES` podium column. The packet-owned heightmap and
  client block-update path must remain distinct from `MOTION_BLOCKING` when
  player-placed leaves make their columns diverge.
- `RabbitModel`'s random idle-head animation starts from client-local entity RNG;
  wire state cannot recover a particular official-client phase, so closure
  requires a source-backed `not-applicable` edge rather than a fabricated seed.

Historical ledger wording is not proof of a gap. Every candidate is checked
against current production code and these local sources before editing.

## Ordered Stages and Focused Tests

### A. Reverse animation inventory

Use the generated renderer presentation owner plus production model selection
to partition every fixed type into avatar model, entity model, typed non-model
pass, or Vanilla noop. Reverse tests must reject raw fallback kinds and require
every production model kind to reach an explicit posed or source-proven static
model implementation. The inventory records the authoritative renderer/model
classes and does not expose a new runtime dispatch API.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world renderer_catalog
scripts/cargo-dev.sh test -p bbb-native entity_model_kind_matches_every_fixed_renderer_presentation_owner
scripts/cargo-dev.sh test -p bbb-renderer entity_model
```

### B. Source-backed family corrections

Fix rejected or known-incomplete families through their existing owner, hardest
ready first. The initial order is Armadillo shell/baby keyframes, baby Axolotl
state-driven keyframes, then Creaking teardown presentation. Continue with any
additional failures found by the reverse inventory. Each family lands as one
buildable commit with exact keyframe counts/timestamps or formula/order tests and
mesh/part assertions that prove the intended bones moved.

Focused tests are the affected family filters in `bbb-world`, `bbb-native`, and
`bbb-renderer`, plus the reverse inventory tests. Random behavior uses fixed
samples only where Vanilla exposes a reconstructable seed; otherwise it receives
the narrow classification above.

### C. Coverage closeout and delivery

Expand `animation_audit` with production implementation, semantic-test, and
Vanilla anchors. Remove completed/misfiled TODOs, keep true equipment/event/tick
gaps in their owning categories, regenerate `docs/entity-coverage-26.1.json`,
and append the milestone index. No animation cell remains `partial` or
`deferred`; source-proven noop/RNG-only exceptions may be `not-applicable`.

Run:

```text
scripts/cargo-dev.sh gate
scripts/playable-gate.sh
```

## Checkpoints, Failures, and Safe Stops

The baseline is fully gated. Stage A and each family correction are
mechanically reviewable commits. Run the full gate after at most four commits,
before changing semantic domains, and at final closeout. A focused failure is
fixed forward; `batch-gate` is used only when the introducing commit is unclear.

Safe stops are limited to a clean reverse-inventory checkpoint, a complete
world/native/renderer family correction, or a documented external blocker with
the prior behavior intact. Do not land a world animation clock without its
native projection and posed renderer consumer in the same family checkpoint.
