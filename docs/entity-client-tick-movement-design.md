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
fluid/collision maps for worlds containing experience orbs. Retained
scratch/spatial narrowing may optimize that cost, but cannot drop entity
collision or change equal-distance Player ordering.

## Phase 3 FishingHook Milestone

FishingHook now has one narrow canonical movement component rather than a
parallel entity schema. It owns the Vanilla `FLYING` / `HOOKED_IN_ENTITY` /
`BOBBING` state, owner UUID, hooked-target UUID plus incarnation, biting/left-owner flags, and the
two Java RNG cursors. A store-local monotonic incarnation token preserves
Vanilla's object-identity-based repeated-deflection suppression across same-id
and same-UUID remove/re-add without entering `EntityState` or creating a
parallel membership table. UUID capture occurs when AddEntity or ordered metadata is
applied, so protocol-id reuse cannot silently retarget an old hook. Same-id
recreation resets or removes the component by generated EntityType policy, and
publication clone copies it into the old immutable generation. The incarnation
is a field of the existing private `EntityIdentity` component, not a separate
hecs column; AddEntity replacement, state replacement, and local-player respawn
all allocate a new value. An intermediate separate-component prototype regressed the valid
hook tick+publication benchmark by about 1.86% and was removed.

The tick follows local Vanilla 26.1 `FishingHook.tick`,
`onSyncedDataUpdated`, `ProjectileUtil.getHitResultOnMoveVector`,
`CollisionGetter.clipIncludingBorder`, `BlockGetter.traverseBlocks`, and
`Entity.move`: interpolation precedes water sampling, hooked targets follow
`getY(0.8)` only while alive/interactable, water entry and Bobbing keep float/RNG
order, and the entity-only benchmark path predicts each successive tick's
pre-`tickTime` value without taking clock ownership away from `WorldStore`.
Collision runs before gravity,
movement precedes rotation, and inertia is last. Block hits use collision-shape DDA rather than whole cells. The shared
narrow target batch preserves owner-root/left-owner, generated pickability,
marker/spectator/living gates, ItemEntity's override, age-scaled hit margin,
and all eight EnderDragonPart bounds. World-border hits run FishingHook's
reverse deflection and -0.1 velocity damping; entity deflection membership is
generated from Vanilla's `#deflects_projectiles` tag, so the Breeze path also
emits its ordered hostile deflect sound without a hand-written type table.

The target batch is built only when at least one hook resolves a current Player
owner. An early prototype built it for invalid-owner fixture hooks and regressed
the 2,000 heterogeneous tick from about 3.315 ms to 4.060 ms. Exact owner
admission removed that scan. Same-machine Criterion after the complete hook
path measured:

| 2,000 heterogeneous case | Item/Orb milestone | FishingHook milestone |
| --- | ---: | ---: |
| tick only | 3.3150 ms | 3.1832 ms |
| tick + immutable publication | 4.6342 ms | 4.5525 ms |

Repeated final reruns recovered the publication regression from the discarded
separate-component prototype and kept all centers below the recorded Item/Orb
milestone; the last standard tick+publication comparison remained within
Criterion's noise threshold. Against the slice baseline, the current result remains a 3.17x tick-only and 2.45x
tick+publication improvement. FishingHook therefore adds the required client
state machine, world facts, collision, deflection, sound, and generation
isolation without transferring a full target scan into the common heterogeneous
tick. The roughly 1 ms full EntityStore publication shell remains the largest
publication-side hotspot. A separate repo-native fixture with one valid
Player-owned hook among the same 2,000 heterogeneous entities measures 4.0581 ms
tick-only and 5.4191 ms tick+publication. Its phase observation attributes
about 1.397 ms to preparation, 0.217 ms to schedule, 0.242 ms to projectile,
0.016 ms to interpolation, and 2.125 ms to animation. The additional one-time
narrow target batch is therefore visible and bounded once per tick, not once
per hook; a retained spatial index is the next optimization boundary if valid
hook density or entity count makes it material. Server fishing simulation
(`catchingFish`, loot, open-water decisions, durability/pull results) and
canonical removal stay with the server/packet authority contract rather than
this client tick owner.

The closeout scheduler now executes every catch-up substep in Vanilla
`Minecraft.tick` order: `ClientLevel.tickEntities` reads the current clock and
border, then the same normally-running substep advances `WorldBorder.tick`
followed by `ClientLevel.tickTime`. Frozen Player/passenger exceptions still
receive raw entity ticks, while the frozen-step budget is consumed once by the
combined `WorldStore` owner. This removes both the former first-tick +1 clock
error in FishingHook synchronized RNG and the aggregate moving-border advance.
An entity-only benchmark API retains the same per-tick inputs without mutating
clock/border state; runtime cannot use that path. Focused tests pin a three-tick
shrinking-border FishingHook collision and a frozen three-tick batch with one
running level tick.

## Phase 4 Locally Controlled Boat Physics Stage

The first local-vehicle stage replaces the previous elapsed-time-scaled boat
approximation with discrete Vanilla 26.1 `AbstractBoat.tick`, `getStatus`,
`floatBoat`, `controlBoat`, and `Entity.move` math for the direct locally
controlling first-passenger root boat. A private narrow `EntityBoatMovement`
component owns only retained Java fields: nullable current/old status,
water-level and land-friction samples, last vertical velocity, accumulated turn
velocity, and underwater control time. Java's initially-null status is
preserved; spawning in water therefore does not incorrectly execute the
air-to-water transition branch.

Status sampling distinguishes source and flowing water, the bottom-surface
contact scan, exact collision-shape land friction, and air in Vanilla order.
Buoyancy, gravity, damping, turn acceleration, paddle booleans, and the
air-to-water collision-guarded snap retain Java float/double order. Horizontal
control uses the shared Vanilla `Mth` sine table reconstruction. Block movement
clips against existing voxel collision shapes, retains the supporting block,
applies ordinary/bed/slime vertical collision response, and applies the exact
generated `BlockBehaviour.Properties.speedFactor` overrides after movement.
The block membership generator now derives both 26.1 speed-factor overrides;
boat and local-player movement consume that single catalog instead of parallel
Soul Sand/Honey Block lists.

The next buildable stage extends that same `Entity.move` pass with the generated
entity collidability/pushability catalog and the exact world-border collision
shape. One narrow hecs batch is projected once per catch-up call; each substep
filters spectators and entities sharing the boat's root vehicle before resolving
entity, border, and block shapes in Vanilla axis order. The air-to-water
`noCollision` guard uses the same entity and border facts, so the snap cannot
cross a canonical collider. This does not add a parallel family table or a full
`EntityState` projection.

Publication cloning copies the narrow component into the immutable hecs shell.
Same-id boat recreation resets it, type reuse removes it, and retained A/B
tests cover transform plus private physics state. The standard 2,000-entity
benchmark remains the end-to-end guard; a dedicated repo-native benchmark now
separates local boat physics from physics plus a retained publication. On the
same machine, the new no-gravity moving-boat fixture measures 1.1205 us for
physics only and 5.8637 us for physics plus retained immutable publication.
After exact entity/world-border collision was added, the same fixture measured
1.3802 us and 6.1161 us respectively: +23.3% for the isolated physics call but
only +4.46% end to end. The absolute increase is the narrow collision-target
query required by Vanilla semantics, not deferred publication work; a later
phase should reuse collision scratch/candidates when the broader local travel
pipeline has a single lifetime owner.
Focused confirmation measured 3.2278 ms tick-only and 4.7282 ms
tick+publication for 2,000 heterogeneous entities, while the 1,000-boat
passenger case remained within Criterion's noise threshold at 3.6756 ms. The
preceding full run's non-boat Zombie controls also moved upward by roughly 2-3%,
so the narrow component did not show an attributable boat-density publication
regression. Final phase-4 comparisons must still be collected under a stable
machine load.

The next stage completes `AbstractBoat` paddle-clock and paddle-audio ordering.
Remote/uncontrolled boats advance the metadata-driven paddle clocks in the
ordered entity query. The direct locally controlled root boat defers that one
narrow mutation until after same-tick `controlBoat` and `move`, so starting and
stopping a paddle cannot lag the input owner by one client tick. The pre-
increment `PI / 4` crossing emits an ordered narrow sound effect; effect
submission then re-evaluates Vanilla `getStatus()` against the post-move box and
selects water, land, or silence. Side offsets, neutral source, volume and the
0.8--1.2 pitch range match `AbstractBoat.tick`; silent boats do not enter the
lossless effect FIFO. UUID/tick-derived presentation randomness follows the
existing reproducible client-local convention because the constructor RNG seed
is not reconstructible from the wire.

The dedicated Criterion now drains and recycles movement effects before each
publication, matching runtime order. Without that drain, the benchmark itself
accumulated an unbounded pending FIFO and incorrectly charged every retained
snapshot for all prior paddle events. On the same machine, the final corrected stage
measures 1.5319 us physics plus effect submission and 6.2547 us for physics,
effect submission, and retained publication. Relative to the exact-collision
stage's 1.3802/6.1161 us this is +11.0% isolated and +2.27% end to end. The added
bounded cost is the required paddle state/effect work; it is neither an
ever-growing publication payload nor deferred to the next mutation. A wider
four-component hecs mutation prototype measured 1.7618/6.6617 us and was
discarded; combining borrow syntax did not reduce the end-to-end owner cost.

The following boat stage closes the remaining remote status and local-client
push ownership without adding another entity scan. The one schedule builder now
collects fixed boat hull inputs in root/passenger order; `WorldStore` resolves
water/land status and scoreboard collision facts before the mutable ECS pass.
The ordered pass applies status before animation, clears remote packet velocity
after interpolation, advances paddles, then applies `ClientLevel`'s sole local-
player push target with exact inflated bounds, vertical, team, spectator,
climbable, shared-root and vehicle gates. Terrain-free fixtures short-circuit to
`IN_AIR`; loaded terrain still performs the required per-boat water/friction
query. Retained A/B tests cover the private boat state, transform impulse and
separate local-player pose owner.

An initial correct prototype rebuilt the schedule and raised the existing 1,000
boat + 1,000 passenger retained-publication case from its historical 3.6756 ms
to about 4.46 ms. Folding hull collection into the existing schedule and using
an ordered fact vector instead of a hash lookup reduced the same-machine result
to 3.9589--4.1157 ms; the latest center was 4.0588 ms. The remaining roughly
10% over the historical pre-status baseline is the newly required mutation of
1,000 persistent boat status/velocity owners, not a second scan or deferred
publication work. The standard 2,000 generated heterogeneous mix measured
3.7690 ms tick-only in the same final run; its latest adjacent comparison was
+1.67%, with only the generated boat rows paying status work. Generic
`applyEffectsFromBlocks` ordering, including bubble-column velocity and
stuck/fire/freeze callbacks, remains the final common base-class boundary and
is not implied complete by this boat stage.

The same phase also closes the client-owned part of `MinecartTNT.tick`. Event
10 already seeds the canonical 80-tick fuse; every tick whose pre-decrement fuse
is positive now decrements it and emits one typed smoke effect at the cart's
post-behavior position plus `y + 0.5`. The existing lossless movement-effect
FIFO drains before publication and resolves the smoke through the common
particle owner. Fuse-zero explosion, horizontal-collision ignition, removal,
damage, and game-rule outcomes remain server-only. A repo-native batched
Criterion keeps all 2,000 carts primed without measuring fixture recreation:
tick/effect drain changed from 1.4323 to 1.4398 ms and the retained-publication
path from 3.4485 to 3.4937 ms (+0.52%/+1.31%); Criterion detected no significant
change in either case. Thus the 2,000 required effects are bounded and do not
become retained snapshot payload.

`MinecartFurnace.tick` is now closed at the same subclass boundary. The server
continues to own fuel countdown, push and rail slowdown; the client reads only
the generated `DATA_ID_FUEL` accessor and, while fueled, advances one narrow
client-local Java-LCG cursor for `nextInt(4)`. A zero result emits typed
`large_smoke` at the post-behavior cart position plus `y + 0.8`. The cursor is
initialized per fresh UUID/incarnation, cloned with immutable generations,
cleared by normal entity replacement/reset, and excluded from JSON/probe
shape. Unfueled ticks do not consume it. With 2,000 fueled carts, the
repo-native tick/effect-drain benchmark changed from 1.4128 to 1.4642 ms
(+3.64% for roughly 500 required effects per tick); retained publication
changed from 3.4939 to 3.4689 ms and had no statistically significant change.
This localizes the cost to effect generation rather than publication or the
next mutation.

The next living-subclass stage closes the client-observable part of
`Blaze.aiStep`. An airborne Blaze now applies the exact downward-velocity
`0.6` damping before its ambient effects. Its canonical client-local Java-LCG
cursor performs `nextInt(24)` first, consumes the two sound floats only when
that branch succeeds and the synced silent bit is clear, and then consumes
three doubles for each of the two mandatory `large_smoke` positions. The burn
sound is queued before one typed two-particle batch, preserving Vanilla order
while avoiding two maximum-sized FIFO owners and two renderer template
lookups. The RNG cursor is cloned with immutable publications, cleared by the
normal incarnation/reset path, and omitted from stable probe serialization.

This stage also records why a narrow family query matters. Before the missing
effects were added, the 2,000-Blaze benchmark measured 3.1674 ms for
tick/effect drain and 4.6844 ms with a retained publication. A first exact but
general implementation called the complete model-bounds projection for every
Blaze and measured 4.8453/6.5781 ms. Replacing that unrelated family ladder
with generated Blaze dimensions times the canonical SCALE attribute reduced
the implemented path to 3.6326/5.2198 ms (25.0%/20.6% below the prototype).
Relative to the behavior-free baseline, the remaining 14.7%/11.4% is the cost
of generating 4,000 required particles plus roughly 1-in-24 sounds, not a
publication rebuild or deferred next-mutation copy. The separate tick-only
measurement is 3.6678 ms; effect draining itself is below the run's noise.

The following ambient-particle stage shares one narrow implementation between
`EnderMan.aiStep` and `Endermite.aiStep` without introducing a hand-written
family table. Each entity owns a client-local Java-LCG cursor, and each tick
consumes the exact six doubles per particle for two `portal` particles. The
position calculation uses generated dimensions times the canonical SCALE
attribute; only Enderman applies Vanilla's `-0.25` Y offset. One typed batch
owns both exact positions and velocities, so the lossless FIFO and native
resolver preserve particle order while performing one sprite-template lookup.
The cursor is cloned with immutable publications and reset with the entity
incarnation, while stable probes continue to omit this presentation-only
state.

Before adding the missing effects, a heterogeneous 1,000-Enderman plus
1,000-Endermite fixture measured 3.2399 ms for tick/effect drain and 4.7202 ms
with a retained publication. The implemented path measures 3.3377 ms and
4.9143 ms: +3.02% and +4.11% by the centers, while Criterion found no
statistically significant retained-publication change. The bounded delta is
the exact RNG and 4,000 required particle payloads; the benchmark drains the
FIFO before publication, so the result does not hide retained-generation or
next-mutation work. The maximum pending logical payload still scales with the
number of ambient entities until the runtime drains the lossless effect FIFO;
batching two particles per entity halves owner/template overhead but does not
remove that explicit peak-memory boundary.

The Guardian ambient/dry stage implements the common non-target half of
`Guardian.aiStep` for Guardian and ElderGuardian from the generated family
membership. The existing in-water projection now also carries an optional
`blockPosition().below()` only while vertical velocity is negative; WorldStore
resolves the loaded collision shape's full UP face before the mutable ECS pass.
That preserves Java's short-circuit and avoids either a second ECS scan or
unconditional chunk probes. One per-incarnation Java-LCG cursor owns the
constructor tail draw, dry spike float, moving-water bubble doubles and dry
ground jump floats in Vanilla order. The cursor and `clientSideTouchedGround`
clone into immutable generations and reset on same-id recreation.

Moving submerged Guardians submit one typed two-bubble FIFO owner behind the
pre-tick view vector. Dry upward motion emits the Guardian/Elder flop sound only
after a remembered full support face and honors the synced silent bit. A dry
grounded tick then applies the three random draws to delta movement/body yaw,
clears `onGround` and its canonical supporting-block component, after all
earlier client effects for that entity. Water air supply is not published and
server AI/pathing/damage remain outside the client owner.
Powder snow's entity-context exception remains conservative because remote
fall distance is not synchronized; that rare flop-arming edge is recorded in
the unsupported ledger rather than guessed.

The repo-native pre-change baseline was 4.4219 ms for 2,000 dry Guardians,
4.5640 ms for 2,000 moving submerged Guardians, and 4.4359 ms for the active-
target fixture; retained-publication variants were 5.6563/5.7317 ms. After the
stage and short-circuit audit, centers are 4.7186/5.0540/4.7374 ms and
6.1862/6.0928 ms respectively (+6.7%/+10.7%/+6.8% and +9.4%/+6.3%). The delta
is the newly required RNG, collision fact and up to 4,000 bubble commands; the
effect FIFO is drained before snapshotting, so it is not deferred publication
or retained-generation work.

The following active-target stage resolves the synced target through one
protocol-id lookup per unique id after direct movement, then rejects non-living
targets through the generated LivingEntity family before reading narrow
transform/dimensions. Guardians sharing one target therefore do not repeat the
lookup or project a full `EntityState`. The client-side Guardian path has a
fresh `PathNavigation` whose path remains null because `serverAiStep` (the only
navigation tick/path owner) is skipped in a `ClientLevel`; consequently the
explicit `LookControl.tick` takes its `navigation.isDone()` branch without the
server path clamp. The implementation resets pitch, rotates pitch/head yaw by
at most 90 degrees, emits the exact Java-LCG beam-bubble sequence from eye to
target mid-height, and finally copies head yaw to body yaw even when the target
id names a missing/non-living entity. Ambient bubbles precede beam bubbles in
the same lossless FIFO.

Immediately before that stage, the 2,000-Guardian shared-target fixture measured
4.7374 ms for tick plus effect drain and 6.0928 ms with a retained publication.
The exact look/beam path measures 6.4886 ms and 7.7427 ms after reserving the
mathematical upper bound for each beam owner (+37.0%/+27.1%). This deliberately
wide 64-by-32 fixture generates tens of thousands of required bubble positions
per tick; the delta is inside tick-side target math, RNG and payload generation,
not the snapshot. Upper-bound reservation removes repeated growth but only
recovers roughly 1--3%, confirming that moving generation to the particle
resolver would merely transfer the dominant work. Peak pending logical memory
remains linear in total same-tick beam length until the effect gate drains it.

The first non-boat local-authority stage implements the directly controlled
Happy Ghast branch without widening the owner model. The generated Happy Ghast
metadata accessor, Body equipment and first-passenger relation decide control;
the generated `flying_speed` and local-player `sneaking_speed` attributes feed
the narrow movement calculation. `LocalPlayer.applyInput` is reproduced with
float cardinal/diagonal normalization, its `0.98` scale and square-movement
correction before `HappyGhast.getRiddenInput`. Each client tick then applies
the exact `0.08` wrapped yaw approach, half controller pitch, Java-table trig,
`moveRelative`, water/lava/air drag and the existing block/entity/world-border
collision owner. The transform and supporting block are written through one
two-component hecs borrow. No full `EntityState` projection, parallel vehicle
table or persistent derived owner is introduced.

Native input preserves passenger-rotation-before-vehicle-report ordering and
uses the existing bounded `ServerboundMoveVehicle` queue; Happy Ghasts never
emit boat paddle packets. A retained A/B publication test covers transform
isolation, and controller-gate tests cover missing harness, still timeout,
focus and backward-pitch input. The repo-native benchmark measures 4.1985 us
for one physics tick and 7.9715 us for physics plus a retained publication on
the same machine (30 samples, one-second warm-up, three-second measurement).
This is a newly implemented semantic path rather than a before/after speedup;
the measurement fixes its mutation-plus-publication budget and shows the
retained snapshot adds about 3.77 us instead of hiding work in the next tick.
The following collision-foundation stage extends the shared locally
authoritative `Entity.collide` owner with source-aware block collision context
and Vanilla's sorted candidate-height step-up search. It reuses the one narrow
entity-collider batch, recollects only block/world-border shapes intersecting
the step sweep, skips the already-clipped vertical height and returns the first
candidate that improves horizontal distance. Boats pass a zero step height;
controlled Living vehicles pass their exact maximum. A grounded Happy Ghast
one-block test proves the new path while the existing wall, entity, border and
boat suites guard the non-step cases. Block-inside/emission callbacks and the
other living rideable families remain part of phase 4 rather than being
approximated here.

This is a buildable stage within phase 4, not closure of locally controlled
vehicles. The existing runtime still applies input/boat movement in its
post-entity-tick input owner, as required by the frozen integration boundary.
Remote persistent boat status, block inside/emission callbacks, and the exact
passenger push pass remain for the subsequent stage. Non-boat
controlling-passenger travel and the per-substep client-level scheduler are now
implemented. Vanilla's client bubble-column branch only advances the
already-modelled wobble multiplier/angle; ejection and velocity changes are
server-only, so no separate client bubble motion remains.
None of those server-authoritative consequences is synthesized here.

The ground-rideable foundation now closes the non-dash, non-jump
`AbstractHorse` branch. Direct local-player control still comes from the
generated ancestry plus saddle/first-passenger gate. The mounted input owner
reuses `LocalPlayer.applyInput`'s float normalization, slow-movement attribute,
`0.98` scale and unit-square correction, then applies AbstractHorse's half
strafe, quarter-speed non-forward movement, controller yaw and half pitch.
`LivingEntity.travel` uses narrow movement-speed, step-height, gravity and
water-efficiency attributes, narrow levitation/slow-falling/dolphins-grace
lookups, generated block friction/speed factors, fluid currents, collision and
supporting-block state. Skeleton Horse's `0.96` water slowdown is the only
subclass override in this branch.

`EntityTypeTags.CAN_FLOAT_WHILE_RIDDEN` is now another generated core-registry
catalog bit sourced from Vanilla's data tag. Horse travel therefore does not
redeclare the six-member tag and future Camel travel consumes the same source.
Air, water/lava, standing refusal, one-block step-up, controller rejection,
native passenger-rotation/MoveVehicle order and retained A/B generations have
focused tests. The repo-native benchmark measures 3.6783 us for one moving
Horse physics tick and 7.8025 us for physics plus a retained publication on
the same machine. This is newly implemented client work rather than a claimed
speedup over the absent path. Player jump-pending scale/stand sliding and Camel
pose/dash cooldown remain retained-state stages; Pig/Strider boost and
Nautilus travel also remain before phase-4 closure.

The next ground-rideable stage adds the non-Camel `PlayerRideableJumping`
release boundary without creating a vehicle-input side table. The existing
canonical `EntityClientAnimationState` now owns the private pending jump
scale, stand-sliding permission and locally started stand counter; immutable
publications clone that state while its stable serde/probe shape remains
unchanged. `LocalPlayer` release first applies `AbstractHorse.onPlayerJump`,
including Vanilla's non-zero minimum `0.4` scale for command data `0`, and
then queues `START_RIDING_JUMP` even for that zero value. The ridden tick
consumes the scale only while grounded, uses the generated jump-strength
attribute, shared exact Honey Block jump factor and narrow Jump Boost lookup,
and applies Java-table yaw trig for the forward impulse before Living travel.
The local stand flag/counter allows movement during the rearing window and is
cleared after 20 ticks; a server-cleared flag cancels the local allowance.

Focused tests cover same-tick release/consumption, packet-before-MoveVehicle
ordering, zero-charge behavior, retained A/B publication isolation and same-id
recreation. This keeps jump mutation in the canonical entity owner and adds
no full `EntityState` projection or new publication payload. Camel's refusal,
pose transition and dash/cooldown remain the immediate subclass stage; this
commit is therefore a buildable phase-4 stage rather than the local-vehicle
milestone boundary.

The Camel subclass stage now consumes that retained release state without
changing the single-owner boundary. Generated Camel/CamelHusk metadata
accessors provide `DASH` and `LAST_POSE_CHANGE_TICK`; the generated
`CAN_FLOAT_WHILE_RIDDEN` catalog and the existing narrow attributes provide
the remaining class/tag facts. `getRiddenInput` and `getRiddenRotation` honor
the sitting/52-or-40-tick transition refusal before `tickRidden`, so forward
input can stand a settled sitting Camel while the current tick still travels
with zero input. The post-stand collision/fluid probe resolves fresh standing
dimensions rather than retaining the sitting AABB.

Grounded local release applies Vanilla's `22.2222 * charge * movement_speed *
block_speed_factor` horizontal dash and `1.4285 * charge * jump_power`
vertical impulse, sets the canonical DASH metadata and retains the end-of-tick
54-tick cooldown. Sprint adds `0.1` movement speed only at cooldown zero.
The common entity tick owns remote/uncontrolled dash clearing, dash-ready
sound and sitting-in-water instant stand; the directly controlled Camel
defers clear/instant-stand until after its ridden travel, preserving subclass
ordering. Ordinary stand emits the Camel/CamelHusk neutral sound at the
AbstractHorse `0.8` volume with the exact adult/baby two-float pitch shape;
the deterministic client-local cursor is cloned with immutable generations.
No protocol-id membership table or second vehicle state is introduced.

Focused tests cover release/packet/MoveVehicle order, exact cooldown, refusal
and stand timing, post-minimum dash clear, ready-sound single emission,
remote clear, remote water instant stand, entity-id recreation and retained
A/B publication isolation. On the same machine, the repo Criterion case
(20 samples, one-second warm-up, two-second measurement) measures 4.6474 us
for steady Camel physics, 8.5837 us for physics plus a retained publication,
and 21.066 us for a fresh dash mutation plus publication while the old
generation remains retained. The retained steady snapshot adds about 3.94 us;
the dash case includes metadata, animation/cooldown, collision and COW work
rather than transferring it to the following mutation. Pig/Strider boost,
Nautilus ridden travel and the remaining subclass inventory are still phase-4
work, so this is the next buildable local-vehicle milestone rather than
coverage closeout.

The AbstractNautilus stage now consumes the same canonical retained
`PlayerRideableJumping` state for Nautilus and ZombieNautilus. Generated class
membership and the generated DASH accessor select the family; no protocol-id
table, alternate metadata schema, or vehicle side map was added. Direct local
control applies the controller's strafe and pitch-derived forward/up input,
half-pitch and half-yaw approach, water/land ridden speed, exact look-vector
dash momentum, collision-clipped travel, and the water-specific `0.9`
resistance. Because Vanilla returns `isPushedByFluid() == false`, this path
deliberately probes water/lava without adding a fluid current.

The canonical client animation owner retains the private 40-tick dash
cooldown and instance-local Java RNG cursor across immutable publications.
Both synchronized DASH values seed a settled cooldown after the first tick,
matching `onSyncedDataUpdated`; local and remote ticks keep five full dashing
ticks, clear DASH before decrement, emit the exact adult/baby neutral ready
sound at zero, and then evaluate the water bubble with Vanilla's RNG order and
clamped mouth pitch. The directly controlled Nautilus defers this subclass
stage until its ridden movement has completed, while remote/uncontrolled
instances remain in the common ordered entity pass. Server-side Breath of the
Nautilus refresh, AI, pathing, damage, inventory outcomes, and other authority
are not synthesized.

Focused tests cover water and land travel, local and synchronized dash
boundaries, the false-valued metadata callback, ready-sound selection/silence,
bubble origin, packet-before-MoveVehicle ordering, same-id recreation, and
retained A/B generation isolation. On the same machine, the repo Criterion
case (20 samples, one-second warm-up, two-second measurement) measures
3.8731 us for steady Nautilus physics, 8.0037 us for physics plus a retained
publication, and 21.112 us for a fresh dash plus retained publication. The
roughly 4.13 us steady retained-snapshot increment is paid in the current
operation rather than deferred to the following mutation. These are absolute
costs for newly implemented client work, not a speedup claim.

The Pig/Strider stage keeps synchronized boost duration in generated metadata
and retains only Vanilla `ItemBasedSteering`'s private boosting flag and
post-increment elapsed cursor in the canonical entity animation owner. Pig and
Strider membership, accessor index and default come from the generated nearest
field schema; the controller gate uses the existing saddle profile, first
Player passenger and exact held steering item. Remote controllers are resolved
from the stable protocol-id/hecs index. The check is fused into the existing
ordered scheduled-entity loop, so this stage adds no second ECS scan, full
`EntityState` projection, temporary owner `Vec`, or per-tick controller map.
Direct local control remains deferred to the native input owner and reports
the resulting root transform through the established MoveVehicle ordering.

Both families use the fixed `(0, 0, 1)` ridden input and controller yaw/half
pitch. Pig applies `movement_speed * 0.225 * boost`; Strider applies
`movement_speed * (0.35 cold | 0.55 warm) * boost`, with the named
`minecraft:suffocating` transient attribute modifier and synchronized render
flag updated as one COW-isolated canonical mutation. The Strider path also
uses exact lava contact, source-lava half-block collision context, and
post-`super.tick` floating without making the renderer or input thread another
world owner. Server AI, temptation/panic, breeding, damage and boost initiation
remain server-authoritative.

Focused tests cover generated membership, local Pig boost boundaries, remote
hand/controller loss, local and remote Strider suffocation order, source-lava
surface collision, packet-before-MoveVehicle behavior, same-id recreation and
retained A/B generations. On the same machine, the repo Criterion case (20
samples, one-second warm-up, two-second measurement) measures Pig at 3.6957 us
physics-only and 8.2662 us with retained publication, Strider at 3.9554 us and
8.7258 us, and a fresh Pig boost plus retained publication at 22.190 us. These
are absolute costs for newly implemented Vanilla work. The steady retained
snapshot adds about 4.57--4.77 us in the current operation rather than shifting
it to the next mutation. Remaining phase-4 subclass work is limited to the
separately listed inventory/callback edges before coverage closeout.

## Precise Block-Effect Boundary Follow-up Contract (2026-07-21)

The final common client-tick boundary is Vanilla 26.1
`Entity.applyEffectsFromBlocks`. This follow-up remains part of the existing
high-risk movement engineering slice; it adds no visible feature and does not
move server damage, AI, block mutation, portal outcomes, advancement triggers,
or authoritative removal into the client. On gated HEAD `378299e89`, the same
machine measures the existing 2,000 heterogeneous repo Criterion cases at
3.7833 ms tick-only and 5.3010 ms for tick plus retained publication (midpoint,
10 samples, 500 ms warm-up and two-second collection). These are the before
numbers for the boundary itself; terrain traversal, callback apply and
publication remain separately observed after implementation.

The generated entity metadata row is the sole family dispatch owner. It now
records a source-verified client callback policy: remote Living entities do
not run the `LivingEntity.aiStep` callback, locally authoritative Living
instances do, EnderDragon's non-super `aiStep` remains excluded, boats call
twice, interpolated client minecarts do not call, ItemEntity can reuse its last
movement list, AbstractArrow owns explicit hit segments, detached fireworks
call once, and the remaining directly moving nonliving families call once.
The generator checks the corresponding Vanilla method call counts and super
chain. Fire immunity comes from each `EntityType.Builder.fireImmune()` call;
freeze immunity comes from `#freeze_immune_entity_types`. No protocol-id table,
raw type-id branch, or second metadata schema is permitted.

Canonical per-incarnation block-effect runtime belongs to `EntityStore`.
The small callback state is nested beside the existing supporting-block
physical owner. Movement history belongs to the already generated-policy
`EntityClientMovementCollisionFacts` component, not a second membership
component. One inline list changes role from current to final at the callback;
a new move discards the previous Item final list because that tick cannot take
the last-movements branch. Only rare multi-segment motion uses a
publication-shared heap owner, so the ordinary moving entity allocates nothing
and publication does not defer ECS component creation to the next mutation.
`WorldStore` remains the only terrain probe owner and produces a narrow ordered
evaluation at the exact callback point. The entity owner then applies stuck
speed, bubble/honey velocity and fire/freeze/clear state before
animation/effect publication. Reset, removal, same-id recreation and level
change discard the incarnation state.

The serial checkpoints are: generated policy/immunity facts and contract;
canonical movement history plus precise swept traversal; directly moving
nonliving families; locally authoritative Living/vehicle integration and
typed effects; then coverage/performance closeout. Each stage is buildable and
uses world/native focused tests, formatting, diff/docs checks and full review.
The generated-fact stage and the completed runtime boundary are milestone full
gates. A safe stop is only a committed, fully gated milestone; an unexpected
Vanilla call owner or an unrepresentable exact collision shape is a real
blocker, while implementation size or an actionable test failure is not.

### Swept callback runtime checkpoint

The first runtime checkpoint keeps the immutable terrain owner borrowed while
mutating only `EntityStore`. The supporting-block physical owner carries the
next-move stuck multiplier and client-private fire/fall/powder state; the
conditional direct-collision owner carries the single current-or-final
movement list. Zero and one movement remain inline; only multiple movements
use a publication-shared `Vec`, so an old generation cannot be changed and the
normal path allocates no entity-owned payload. A terrain-free world neither
records segments nor enters the callback query. A 50-sample rerun measured the
existing 2,000 heterogeneous no-chunk tick at 3.6970 ms versus 3.7833 ms before
the midpoint (2.3% faster), and retained publication at 1.0367 ms versus
1.0426 ms. This also verifies that history preservation was not paid as a
first-mutation component rebuild.

The borrowed terrain evaluator ports Vanilla's axis-dependent movement split,
16-step fallback, deflated target box, furthest-corner DDA, per-application
cell dedupe and `InsideBlockEffectType` order. Reused small linear scratch is
faster than hashing for normal entity bounds, while a 16-slot direct-mapped
borrowed chunk cache prevents repeated scans of the up-to-625-column owner.
Block lookup copies only callback, collision-emptiness and fluid facts; it does
not clone `BlockProbe` strings/properties or construct another world. Fluid
height observes same-fluid-above, and bubble-column surface selection accepts
any empty collision shape rather than only air. Direct nonliving families,
ItemEntity last-movement reuse, detached firework gating and the two boat
passes now consume the generated policy. Locally authoritative Living and
locally moved vehicles remain the next serial checkpoint; explicit Arrow
multi-hit segments, rain clearing and client snowflake/extinguish effects
remain open rather than being inferred complete by the generic fallback
segment.

A detached baseline worktree at `24434b87f` and the same Criterion shape
(20 samples, one-second warm-up, two-second collection) measured 2,000 Item
ticks at 2.0893 ms in one loaded air column, 2.0840 ms in cobweb, and 4.2962 ms
for cobweb tick plus retained publication. The first correct runtime measured
3.7724/3.8536/6.2190 ms. Replacing per-entity hash scratch with retained linear
scratch and compacting current/final history produced final centres of
3.7012/3.7590/6.0183 ms, 1.9%, 2.4% and 3.2% below that first correct version.
With 625 retained columns the final loaded-air/cobweb centres are
4.2682/4.1733 ms versus the old no-callback 2.6383/2.6054 ms. The remaining
delta is the newly required swept callback work for all 2,000 applicable
entities, not delayed component creation or publication work; production
diagnostics now separate its recent/peak duration and entity/cell/callback
counts. These results intentionally report the regression against the old
missing-behavior baseline rather than presenting correctness work as a speedup.

### Locally controlled vehicle checkpoint

The directly controlled vehicle owners now run the locally authoritative
callback at their Vanilla slots: the boat applies its two passes after `move`
and before the paddle/sound loop; horse/camel, nautilus, pig/strider and
happy ghast apply one pass after ridden travel completes, before the nautilus
subclass tail and the strider post-`super.tick` float. Each travel records its
single `Entity.move` (clipped segment plus the requested axis-order vector) in
a one-slot scratch recorder that the same call consumes, mirroring
`movementThisTick`; a boat's second pass falls back to the plain
old-to-current segment exactly like Vanilla's cleared recorder. Every local
move point consumes the pending stuck-speed multiplier before collision
clipping and zeroes the post-move delta, so the ridden travel tails apply
friction and gravity to the zeroed vector; the boat's retained
`checkFallDamage` delta is zero in that case. Airborne stores now clear only
the physical supporting-block fields — the private block-effect runtime
survives until the incarnation resets, matching Vanilla state ownership.
Focused tests cover cobweb stuck set-then-consume on a ridden horse and the
boat's double freeze pass in powder snow.

### Arrow segments and rain checkpoint

`AbstractArrowSegments` now runs explicitly from the arrow movement slot
instead of the ordered common pass: the flight branch applies its recorded
`initialPosition → nextLocation` segment between `setPos` and the
`applyInertia(0.99)`/gravity tail (which now scales the post-callback delta,
so bubble/honey delta edits compose like Vanilla), the in-ground branch
applies the degenerate no-arg segment, and the `Entity.baseTick` reset
(positive-fire clear plus powder marker) is placed per branch —
before callbacks for common policies, after callbacks for flight, never for
in-ground. The tick head applies `isInWaterOrRain → clearFire` for every
branch, and the trident return branch mirrors `setNoPhysics(true)` into a
persistent client `noPhysics` field that `isAffectedByBlocks` reads to skip
the callback entirely. `Entity.isInRain` is evaluated inside the evaluator
after `applyAndClear` for all policies: `Level.isRainingAt` over synced sky
light (`canSeeSky` needs 15), the packet `MOTION_BLOCKING` heightmap
(missing final maps behave zero-filled), and registry biome climate through
the world-owned `Biome.getHeightAdjustedTemperature` stack moved from the
renderer glue. Arrows collapse to one segment per tick because the client
does not simulate `stepMoveAndHit` entity-hit stepping. Still open on this
boundary: client snowflake/extinguish presentation.

### Local player aiStep checkpoint

The locally authoritative player now runs `applyEffectsFromBlocks` per pose
physics step (the driver advances exactly one 0.05s step per client tick):
each step consumes the entity-recorded stuck-speed multiplier at its
`Entity.move` point (multiplying the requested vector and zeroing the
post-move delta, replacing the old current-position probe and its
fall-distance reset — the reset now comes from the block callback), then the
post-travel slot evaluates the swept callback with the step's recorded
clipped move against the player entity's private effect state, pose-derived
bounds, and pose-owned position/delta/fall distance. The evaluator owns
freeze (+1 with cap; thaw is `ServerLevel`-gated so the old local −2 decay is
gone), private fire/rain state, powder-snow marker, and the bubble-column and
honey delta edits (the fluid step's local bubble implementation was removed
to avoid double application). `canFreeze` extras (spectator mode,
freeze-immune wearables) fold into the query's freeze immunity, and a
spectator skips the whole callback (`noPhysics` ⇒ `isAffectedByBlocks`
false). Because the pose pipeline derives its delta from moved distance per
step, the post-consumption delta is explicitly zeroed and gravity rebuilds
from zero in the vertical branch, matching vanilla's zeroed
`setDeltaMovement` at consumption.

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
