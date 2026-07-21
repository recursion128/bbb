# Entity Client Tick and Movement

This document freezes the high-risk engineering contract for completing the
Vanilla 26.1 `client tick/movement` coverage cell for all 157 fixed entity
types. It begins at full-gated commit `2b3ae8e736179d912b27ba09174812b383e8d582`,
after the canonical transform/interpolation slice closed its adjacent cell.

## Semantic Goal and Completion

The one simulation owner must run every client-observable entity tick that an
official multiplayer client runs: common tick counters, remote interpolation,
client-owned movement, locally controlled vehicle movement, passenger
`rideTick`, animation timers, and client-only particle/audio/effect triggers.
It must not run server AI or manufacture server-authoritative outcomes.

The authoritative flow remains:

```text
ordered packet state + fixed client-tick deadline
  -> WorldStore / EntityStore canonical narrow components
  -> generated runtime-class tick policy
  -> one ordered client tick and typed effects/outbound commands
  -> immutable publication
  -> renderer and control consume one exact world Arc/generation
```

The slice is complete when every generated EntityType row has an evidence-backed
`complete` or Vanilla-proven `not-applicable` `client tick/movement` cell; all
applicable client-owned movement and subclass tick state is represented by a
canonical component or typed effect; local/server authority and passenger
ordering match Vanilla; the hot path no longer performs every family metadata
lookup for every entity; retained publications remain immutable; and the final
full, current-HEAD live, and policy auto gates pass.

## Non-Goals

- Do not port mob AI, goals, navigation, spawning, breeding, drops, server RNG,
  server collision authority, combat results, projectile hit results, block
  modification, or authoritative removal.
- Do not close animation, event/particle/audio, dimensions/pick,
  equipment/passenger/leash, model/layer, or save/debug cells merely because a
  tick supplies one of their inputs.
- Do not change protocol shapes, tick cadence, input semantics, renderer order,
  visible work, effect-before-publication ordering, or the server authority
  boundary.
- Do not add a second canonical entity world, a native-owned mutable movement
  cache, a hand-written entity-type membership table, raw EntityType numbers,
  shared mutable `WorldStore`, `unsafe`, Rayon, or another concurrent owner.
- Do not claim a performance win by skipping Vanilla-required ticks,
  publication, effects, or diagnostics, or by moving a full scan/clone to a
  later mutation.
- Do not cache partial tick, camera, wall-clock, random presentation output, or
  server-private state in the canonical tick owner.

## Owners and Frozen Boundary

- `bbb-protocol` retains packet and generated fixed EntityType ownership. No
  wire changes are planned.
- `scripts/` derives the tick-owner/classification inventory from the existing
  Tree-sitter Java frontend, `EntityType.java` runtime classes, and resolved
  class graph. Generated output is checked in; normal builds never read local
  Vanilla sources.
- `bbb-world::EntityStore` remains the only canonical entity owner. Narrow
  components own only state that survives between client ticks. Common and
  family queries may mutate those components directly without projecting
  `EntityState`.
- `bbb-world::WorldStore` supplies level/chunk/fluid/collision/session facts and
  receives typed tick results. It must resolve immutable world facts before a
  conflicting mutable ECS borrow, without constructing all-entity projections.
- `bbb-native` owns tick scheduling and converts typed world reports into
  ordered particle/audio/network effects before publication. It never re-runs
  entity behavior or reads live entity state after publication.
- `bbb-renderer` remains a derived consumer. It may interpolate canonical
  endpoints and consume counters but never advances an entity tick.
- Renderer and control continue to share one `Arc<WorldStore>` and publication
  generation. Reset, respawn, dimension/session change, same-id recreation, and
  entity-id reuse discard tick-local state.

Instance protocol ids identify entities only. Entity family behavior is derived
from the fixed type catalog and generated runtime class graph. A hash may speed
lookup but never establishes semantic equality.

## Vanilla 26.1 Evidence

The main agent rechecks the exact source before each owning phase. The initial
contract is based on the local tree at
`/home/zgy/Work/mc-code/sources/26.1`:

- `ClientLevel.tickEntities`, `tickNonPassenger`, and `tickPassenger` tick only
  loaded/ticking, non-removed roots that are not passengers; they snapshot old
  pose, increment `tickCount`, call `tick`, and then recursively call
  `rideTick`. A Player passenger ticks even outside ordinary section
  membership. Frozen entities do not tick.
- `Entity.tick` delegates to `baseTick`. The common client-observable state
  includes first-tick and portal/fluid/fire/freeze/fall-related bookkeeping,
  but server-only damage, block mutation, and authoritative removal remain out
  of scope.
- `LivingEntity.tick` calls `baseTick`, using-item/swim updates, `aiStep`, body
  rotation, and client `calculateEntityAnimation`; `RemotePlayer` supplies its
  client interpolation/head-rotation path. Death ticking is gated by
  `ClientLevel.shouldTickDeath`.
- `InterpolationHandler` and `ClientPacketListener` establish the generic
  remote movement behavior already owned by the preceding transform slice.
  This slice must invoke that owner in exact tick/passenger order, not create a
  second transform path.
- `LocalPlayer` owns prediction and outgoing movement/input ordering. Only the
  exact locally controlling vehicle branch gains client movement and a
  `ServerboundMoveVehicle` report; remote vehicles and mobs stay
  server-authoritative.
- `Projectile`, `AbstractArrow`, `ThrowableProjectile`,
  `AbstractHurtingProjectile`, `FishingHook`, `ShulkerBullet`, `LlamaSpit`,
  `EyeOfEnder`, `ExperienceOrb`, `ItemEntity`, `FallingBlockEntity`,
  `PrimedTnt`, `AreaEffectCloud`, `LightningBolt`, and their concrete
  subclasses define the finite nonliving movement/timer/effect families. Each
  branch must separate unconditional/client work from `ServerLevel` or
  `!level.isClientSide` work.
- Living subclass `tick`/`aiStep` overrides provide client animation counters,
  pose easing, particles, and sounds only where their source executes them on
  the client. An override existing in the class graph is not itself proof that
  its server-only body belongs in this client.
- `Minecraft.tick` and `ClientPacketListener` preserve the existing order:
  queued network state, client tick systems/effects, `ClientTickEnd`, immutable
  publication, then render consumption.

The generated audit must enumerate nearest overrides of `tick`, `baseTick`,
`aiStep`, and `rideTick` for every runtime class. A semantic policy then records
which owner contributes common state, interpolation, client movement, timer,
effect, local-authority, or server-only behavior. It may reference shared
evidence; it may not infer behavior solely from method names.

## Implementation Contract

### Generated classification

The existing metadata generator/class graph is the single source for runtime
class ancestry. The new policy surface must partition all 157 catalog entries
and expose only semantic facts needed by world code. Generated reverse guards
must reject a parallel production type-id list and make a new EntityType or an
unclassified relevant override fail `--check`/tests.

The classification distinguishes at least:

- common `Entity` tick only;
- Living/RemotePlayer common tick;
- generic interpolation handler;
- passenger `rideTick` specialization;
- locally controlled vehicle movement;
- projectile/nonliving client motion;
- family client timer/animation/effect work;
- fully server-only override body beyond inherited client work.

One entity may consume several shared policies. The policy is not a second
behavior implementation and must not copy source expressions that belong in a
typed component/helper.

### Tick order and authority

One tick visits root entities under `ClientLevel` membership, snapshots their
old endpoints once, increments their client age once, advances their exact
tick owner, then recursively visits valid passengers through `rideTick`.
Removed/mismatched passenger links detach through the canonical mount owner.
Player passengers keep the Vanilla always-ticking exception.

Remote interpolation remains packet-target driven. Client motion is applied
only for Vanilla classes whose tick advances motion locally. Locally controlled
vehicles reuse the generated controlling-passenger rules and typed outbound
report; no remote mob or vehicle gains prediction.

Every server-authoritative consequence is either absent or represented as an
incoming-packet result. Client-side visual/audio work becomes a typed report
drained before publication. Random behavior uses the existing deterministic
online/probe/fixture context where reproducibility is required; irrecoverable
unseeded presentation randomness receives a narrow documented
`not-applicable`, never a fabricated canonical value.

### Narrow ECS work

The hot path uses component-family queries. The common pass accesses identity,
transform, mount, and narrow common tick state. Metadata, attributes,
equipment, water/collision, targets, or particle inputs are queried only by
the families that consume them. No tick path builds `EntityState`, clones all
metadata/equipment, walks `order` plus `by_protocol_id` repeatedly, or performs
all schema accessor lookups for unrelated entities.

Stable family policy comes from the generated per-type catalog before large
payload access. Temporary maps/vectors are retained scratch where an
order-independent cross-entity relation is necessary, cleared and reused, and
bounded by tracked entity count. A family needing one target lookup uses the
stable protocol-id → hecs handle index; it does not scan every entity for every
consumer.

Logical observation records per generation include scanned/ticked/skipped,
common/family/movement/effect-mutated counts and recent/peak phase durations.
They remain low-overhead counters, not high-frequency logs or allocator hooks.

### Publication and lifecycle

Only actually mutated canonical state dirties the world generation. A tick
that legitimately increments client age is productive; a frozen/no-entity
tick with no other state/effect follows the existing no-op publication rules.
World-only diagnostics generations reuse the exact world Arc. Effects become
visible before the publication they describe.

Retained generation A must not change when B advances common state, projectile
motion, local vehicle motion, passenger state, family metadata-derived timers,
spawn/remove, or reset. Entity id reuse cannot inherit a previous instance's
tick component or scratch entry. Level/session/resource reset clears or
invalidates every instance-keyed owner while bounded scratch capacity may stay
render/simulation-owned without semantic contents.

## Reproducible Baseline

Command, on the same machine and current source before runtime changes:

```text
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo bench -p bbb-world \
  --bench publication_snapshot -- entity_tick --noplot
```

The benchmark fixture was first repaired to use three generated, serializer-
correct Zombie Byte accessors instead of invalid Byte values at metadata slots
0..7. This changes only the payload-heavy benchmark fixture and makes schema
validation part of the setup contract.

Criterion medians at `2b3ae8e73` plus that fixture repair:

| case | median |
| --- | ---: |
| empty tick only | 41.605 ns |
| 100 Zombie tick only | 493.45 us |
| 500 Zombie tick only | 2.4793 ms |
| 2,000 Zombie tick only | 9.9706 ms |
| 2,000 heterogeneous tick only | 10.093 ms |
| 2,000 Zombie 17-accessor probe | 266.07 us |
| 2,000 heterogeneous 17-accessor probe | 259.19 us |
| empty tick + publication | 2.6932 us |
| 100 Zombie tick + publication | 526.79 us |
| 500 Zombie tick + publication | 2.6463 ms |
| 2,000 Zombie tick + publication | 10.766 ms |
| 2,000 heterogeneous tick + publication | 11.147 ms |
| 1,000 vehicles + 1,000 passengers + publication | 10.315 ms |
| 2,000 firework trail effect generation | 9.0444 ms |

The isolated accessor loop is only about 0.26 ms; the evidence therefore does
not attribute the roughly 10 ms tick to schema lookup alone. Phase work must
measure query/borrow, cross-entity preparation, common mutation, family
mutation, effects, and publication separately. Wall-clock values are recorded
for comparison, not asserted in CI.

## Phase 2 Cost Attribution and Ordered Schedule

The first runtime stage added retained scratch for the root/passenger schedule
and recent/peak observations for preparation, schedule, projectile,
interpolation, and animation phases. A repo-native Criterion phase observation
reports the average internal phase durations alongside the external total; the
same values are published in `NetCounters` without per-tick logging.

The initial phase measurement showed that the all-entity transform pass, not
hecs borrow or metadata accessor lookup, owned about 5.7-6.0 ms of a 2,000
entity tick. `vanilla_entity_metadata_type_interpolation_steps` was walking the
generated Java class graph several times for every entity on every tick. The
fixed 157-entry catalog now stores the source-derived interpolation and vehicle
policies directly. The client tick decides those narrow policies before large
metadata/equipment payload access and calls family projections only for the
families that consume them.

The same-machine result after that change and the recursive root/passenger
schedule is:

| case | before | phase 2 | improvement |
| --- | ---: | ---: | ---: |
| 2,000 Zombie tick only | 9.9706 ms | 2.9391 ms | 3.39x |
| 2,000 heterogeneous tick only | 10.093 ms | 2.6542 ms | 3.80x |
| 2,000 Zombie tick + publication | 10.766 ms | 4.0052 ms | 2.69x |
| 2,000 heterogeneous tick + publication | 11.147 ms | 3.8887 ms | 2.87x |
| 1,000 vehicles + 1,000 passengers + publication | 10.315 ms | 3.6628 ms | 2.82x |
| 2,000 firework trail effects | 9.0444 ms | 1.5060 ms | 6.01x |

For the after run, the representative internal averages were 5.4 us
preparation, 158 us schedule, less than 0.1 us projectile probing, 14 us
interpolation, and 2.77 ms animation for homogeneous Zombies. The heterogeneous
case measured 55 us, 183 us, 162 us, 16 us, and 2.19 ms respectively. Snapshot
construction remains about 1 ms for 2,000 entity shells and is therefore the
largest remaining publication-side boundary; it was not moved into a later
mutation. The retained schedule stores hecs handles only in simulation-owned
scratch, clears semantic contents every tick, and is excluded from immutable
publication clones.

## Phase 3 Item and Experience-Orb Milestone

The Item/ExperienceOrb world-fact owner now follows the local Vanilla 26.1
`ItemEntity.tick`, `ExperienceOrb.tick`, `Entity.baseTick`,
`EntityFluidInteraction`, `Entity.move`, and `EntityGetter.getNearestPlayer`
sources. One immutable preparation pass samples deflated fluid boxes, eye/block
fluid, block/entity/border collision, supporting blocks, and generated block
friction before the mutable ECS pass. Item movement preserves water/lava float
math, gravity, the `(tickCount + id) % 4` settled-item gate, horizontal
`Mth.equal`, ground drag, and bounce. Experience orbs preserve the first-client-
tick exception, eye-water motion, collision-gated gravity, deterministic Java-
float lava impulses, retained nearest-player identity/order, attraction, drag,
and vertical-collision bounce. Server merge, expiry, pickup, damage, and
authoritative removal remain outside this client owner.

Block friction is no longer approximated: the block membership generator reads
the five non-default `Blocks.java` `.friction(float)` registrations and world
code defaults every other block to `0.6F`. Fluid/collision and exact Player
target membership are zero-state hecs family markers derived on spawn/re-add
from the generated movement policy or Player identity. Same-id type reuse
regenerates them; publication clone derives them from the already-borrowed
identity rather than probing each marker on every entity. The only new
canonical payload is an orb's retained player UUID and deterministic random
cursor, both cloned into an old immutable generation.

The entity `noCollision` path first projects only generated non-`Never`
collidability families, then reuses the existing generated instance predicate
and world-border owner. This replaced an initially measured 7.245 ms
heterogeneous tick+publication prototype that rebuilt full pick targets. The
same-machine Criterion result after the narrow target correction is:

| 2,000 heterogeneous case | start-of-milestone | Item/Orb milestone |
| --- | ---: | ---: |
| tick only | 2.8151 ms | 3.3150 ms |
| tick + immutable publication | 4.0668 ms | 4.6342 ms |

The added roughly 0.5 ms is real client behavior rather than a publication or
next-mutation transfer. Against the slice baseline it remains a 3.04x tick-only
and 2.40x tick+publication improvement. The preparation average is now about
0.54 ms; its remaining hotspot is collidable-target projection plus temporary
fluid/collision maps for worlds containing experience orbs. FishingHook still
needs its separate state machine and this preparation data before phase 3 can
close. Retained scratch/spatial narrowing may optimize that cost, but cannot
drop entity collision or change equal-distance Player ordering.

## Required Tests

At minimum, focused semantic tests cover:

- generated 157-entry tick-owner partition and reverse guards;
- loaded/unloaded roots, always-ticking Player, frozen world/entity, removed
  root, valid/mismatched nested passenger, and one age increment per tick;
- common Living state, head/interpolation order, death-distance gating,
  using-item/swim/walk state, and remote player behavior;
- each projectile/nonliving movement policy, exact gravity/drag/rotation/timer
  order, client particle/audio report, and absence of server-only outcomes;
- local player, controlled and uncontrolled vehicle, correction/outbound order,
  and passenger `rideTick` specialization;
- representative living subclass timer/effect families plus an exhaustive
  policy-to-evidence check;
- same-id recreation, id/type reuse, spawn/remove churn, disconnect, respawn,
  dimension/session reset, and retained-publication isolation;
- effect-before-publication, render/control exact world Arc pairing, blocked
  renderer, shutdown/join, panic propagation, and live fixture behavior;
- before/after tick-only and tick+publication measurements for empty, 100,
  500, 2,000 homogeneous/heterogeneous, passengers, effect-heavy, sparse and
  dense mutation, and retained publication cases.

Each phase runs affected-crate focused tests, `cargo fmt --all --check`,
`git diff --check`, docs lint when relevant, and complete diff review before a
normal commit.

## Serial Phases and Gates

1. **Contract, generated audit, and baseline.** Repair the invalid benchmark
   fixture, record before data, land this contract, and add a read-only
   generated tick-owner/override inventory with deterministic check/tests.
   This phase changes no runtime semantics. Because the baseline exposed an
   abnormal focused run and the slice is high risk, run a full gate at its
   committed tip.
2. **Common/passenger tick and narrow ECS passes.** Implement exact root and
   recursive passenger ordering, common Entity/Living/RemotePlayer state, and
   split the all-family wide query into semantic component-family passes.
   Extend phase observations and rerun Criterion. Commit buildable stages and
   run a milestone full gate no later than four commits. This is the first safe
   stop.
3. **Projectile and nonliving movement.** Implement the generated applicable
   motion/timer families with typed client effects and explicit server-only
   boundaries. Run world/native effect-order and retained-generation tests,
   commit, and full-gate this ownership boundary.
4. **Local vehicle and living subclass tick.** Close locally controlled
   movement/rideTick and remaining generated client-only family timers/effects.
   Preserve current input/outbound ordering. Run world/native/render focused
   suites, commit buildable stages, and milestone full gate.
5. **Coverage and performance closeout.** Regenerate only the
   `client tick/movement` cells with evidence, remove its unsupported TODO,
   archive the slice, collect before/after data, and audit memory/scratch and
   adjacent cells. Commit and run final full gate.
6. **Delivery.** Build `bbb-native` from the final HEAD in
   `/tmp/bbb-target-main`, verify its timestamp, confirm real Weston PID plus
   socket, run the explicit live Vanilla 26.1 gate, and then the policy auto
   gate.

No amend, rebase, reset, or history rewrite is allowed. A full-gate failure is
fixed forward; `batch-gate` is used only if focused diagnosis cannot identify
the introducing commit.

## Failure Localization and Safe Stop

Generated evidence lands before runtime changes. Common/passenger ordering
lands before new movement. Projectile/nonliving movement lands before local
vehicle and living special families. Coverage flips last. This keeps failures
attributable to one owner and prevents a green ledger from preceding behavior.

The first safe stop is a committed, fully gated common/passenger foundation.
Later safe stops require a full-gated projectile or local-authority milestone.
A real blocker is an unresolvable Vanilla semantic dependency or evidence that
invalidates this owner contract; size, benchmark cost, or an actionable gate
failure is not a blocker. Live failures retain their artifact directory and
are classified as environment, stale binary, client exit, control disconnect,
timeout, or semantic assertion before a current-HEAD rebuild and full rerun.
