# Entity Transform and Interpolation Engineering Contract

This document freezes the high-risk engineering slice that closes the
`transform/interpolation` dimension for the fixed Vanilla 26.1 EntityType
catalog. The starting coverage ledger has 157 partial rows, all pointing at the
same generic packet-transform evidence. That implementation preserves protocol
position-codec bases and basic remote lerp, but it does not yet model the full
client-level ticking decision, old/current render endpoints, teleport snap
rules, collision-aware target carry, head interpolation, or every
family-specific authority exception.

The previously completed spawn/default slice supplies an exact generated
runtime-class ancestry for every fixed EntityType. This slice consumes that
catalog instead of introducing a hand-maintained entity membership table.

## Semantic Goal and Completion

For every one of the 157 generated EntityType entries, the canonical client
world must apply Vanilla 26.1 position, rotation, head rotation, teleport,
relative movement, local-authority, passenger, and interpolation semantics.
Render extraction must use the same previous/current transform endpoints and
partial-tick rules as Vanilla. Packet application, the simulation client tick,
renderer extraction, control queries, and immutable publication must remain one
ordered path over one canonical entity owner.

The authoritative flow is:

```text
ordered clientbound transform packet or simulation client tick
  -> EntityStore canonical PositionCodec / transform endpoints / lerp target
  -> immutable WorldStore publication generation
  -> narrow partial-tick render candidate and full render projection
  -> renderer and control observe the same world Arc and generation
```

The slice is complete when every `transform/interpolation` row is `complete` or
has a Vanilla-proven `not-applicable` result; the generated class graph proves
the handler and special-family partition; remote packet decisions match
`ClientPacketListener`; previous/current endpoints drive partial-tick
projection; reset, same-id recreation, entity-id reuse, level generation, and
retained publication isolation are tested; local vehicle recovery preserves
ordered serverbound effects; and final full, explicit live, and policy auto
gates pass.

## Non-Goals

- Do not port remote mob AI, goals, navigation, server physics, damage, combat,
  spawning, drops, or authoritative projectile hit results.
- Do not claim the adjacent `client tick/movement`, animation, dimensions/pick,
  passenger/leash, or model/render coverage cells merely because a transform is
  now exact.
- Do not change packet ids or wire formats, simulation tick cadence, input,
  visible content, renderer order, effect ordering, or the server-authority
  boundary.
- Do not create a second canonical entity world, native-owned mutable transform
  cache, parallel entity-type membership table, numeric EntityType literals,
  shared mutable `WorldStore`, `unsafe`, Rayon, or another concurrent owner.
- Do not cache partial tick, camera, wall-clock, or interpolation output in a
  resource-stable owner. Only canonical tick endpoints and targets may persist;
  per-frame interpolation remains derived.
- Do not make every entity locally simulated. Local authority remains limited
  to the local player and the exact Vanilla controlling-passenger vehicle path.
- Do not treat the existing minecart render interpolation as proof of generic
  entity packet interpolation, or silently replace new-minecart behavior with
  the old generic handler.

## Owners and Frozen Integration Boundary

- `bbb-protocol` continues to own structured packet decode/encode. This slice
  does not change packet shapes or relative-argument masks.
- `bbb-world::EntityStore` remains the only canonical entity owner. It owns the
  protocol position-codec base, current transform, previous-tick render
  endpoints, optional interpolation target, generated handler policy,
  client-level ticking decision, passenger graph, and local-authority decision.
- `bbb-world::WorldStore` owns chunk residency and level generation. The
  `ClientLevel.isTickingEntity` equivalent is derived from the canonical loaded
  chunk set plus Vanilla's always-ticking player rule; it is not a parallel
  membership cache. A query must use the entity's current section/chunk and the
  current level generation.
- `bbb-native` remains the ordered adapter for lossless effects and outbound
  commands. It may emit the already-typed vehicle correction command only from
  a `bbb-world` report, before the corresponding publication becomes visible.
  It must not reread or independently mutate entity transforms.
- `bbb-renderer` owns only derived per-frame projection. Candidate and full
  render extraction consume canonical previous/current endpoints and a supplied
  partial tick without writing them back or cloning a second entity state.
- Control and renderer continue to read the exact same immutable world `Arc`
  and publication generation. Old generations must remain bitwise unaffected by
  live packet, tick, reset, or id-reuse mutation.

The existing fixed EntityType resource and generated ancestry prove family
identity. Hashes may accelerate lookup but may not establish semantic equality.
Level or protocol entity ids are canonical instance identity, never family or
interpolation-policy identity.

## Vanilla 26.1 Evidence

Local source root: `/home/zgy/Work/mc-code/sources/26.1`.

- `ClientPacketListener.handleEntityPositionSync` first updates the entity's
  `PositionCodec` base. A non-locally-authoritative entity interpolates only
  while `ClientLevel.isTickingEntity` and when squared displacement is at most
  4096; otherwise it snaps. An accepted snap also repositions an indirect local
  player passenger and resets that passenger's old pose.
- `ClientPacketListener.handleMoveEntity` always decodes against the position
  codec. Locally authoritative entities update only the codec base. Remote
  position and rotation deltas enter `moveOrInterpolateTo`, including the
  rotation-only form, and then apply `onGround`.
- `ClientPacketListener.handleTeleportEntity` resolves relative values against
  `PositionMoveRotation.of(entity)`, which observes an active interpolation
  target. It interpolates when the entity is ticking, is remote authoritative,
  or the packet has a relative position component, unless displacement exceeds
  4096. Its snap branch calculates old endpoints with the same relative mask.
- A snapped indirect local-player vehicle path repositions and resets the rider
  before the client reports its corrected vehicle position. A missing packet id
  may recover only the removed local-player vehicle path already represented by
  `EntityStore`.
- `ClientPacketListener.handleRotateMob` calls `Entity.lerpHeadTo(yHeadRot, 3)`;
  it does not overwrite both head-yaw endpoints immediately.
- `ClientPacketListener.handleMoveVehicle` accepts only the locally
  authoritative root vehicle. It compares the packet target against an active
  interpolation target when present, cancels a meaningfully different target,
  and snaps the vehicle.
- `InterpolationHandler` keeps a target, remaining steps, and previous tick
  position/rotation. Retargeting an exactly equal target is a no-op. Each tick
  carries entity self-motion into the target only when the shifted target bounds
  remain collision-free; rotation carry is unconditional. Position and yaw use
  lerp/rot-lerp, pitch uses linear lerp, and the handler updates its previous
  sample after applying the step.
- Handler owners are source-derived: `LivingEntity` except Shulker,
  `FishingHook`, `AbstractBoat`, `ExperienceOrb`, `Display` with zero steps, and
  old-minecart behavior. New-minecart behavior keeps its existing track sample
  interpolation instead of gaining the generic handler.
- `ClientLevel` starts/stops ticking ordinary entities with loaded entity
  sections; Player is always ticking. `tickNonPassenger` and `tickPassenger`
  call `setOldPosAndRot` immediately before the entity tick. Passenger ticking
  uses the same Player-or-ticking-membership rule.
- `Entity.snapTo` makes current and old position/rotation equal. Generic entity
  render state linearly interpolates old/current position; living render state
  additionally uses rotational interpolation for body and head endpoints.
- `Entity`, `LivingEntity`, `Shulker`, `Display`, `ExperienceOrb`,
  `FishingHook`, `AbstractBoat`, old/new minecart behavior, and Ender Dragon
  sources define the finite special-family cases. The checked-in generator must
  derive their catalog membership from runtime ancestry rather than copying a
  list into production code.

Critical facts are rechecked by the main agent before the phase that owns them.
Normal build and test never read the local Vanilla tree.

## Frozen Transform Contract

### Canonical endpoints and tick order

`EntityTransform` will retain current position/body rotation plus the previous
tick position and rotation endpoints required by Vanilla render extraction.
Head yaw keeps its own previous/current lerp endpoints. The simulation entity
tick snapshots old endpoints exactly once before the relevant non-passenger or
passenger tick; packet snaps explicitly reset endpoints only where Vanilla does.
Render partial tick is never written into canonical state.

Spawn/recreation initializes old and current endpoints from the recreated packet
state. Successful same-id AddEntity, type reuse, disconnect, respawn, dimension
change, and level reset discard targets and histories. A retained immutable
generation keeps the old endpoints and targets even when the live owner advances.

### Packet target and position-codec semantics

Position synchronization updates `PositionCodec` before authority and ticking
decisions. Relative movement always decodes from that codec. A local-authority
packet may update the codec without changing the predicted transform. Teleport
relative values observe the active target, while their snap old-endpoint values
observe the pre-packet old/current endpoint selected by Vanilla's relative mask.

Distance fallback uses the exact squared-displacement threshold of 4096 and is
tested on both sides. Display's zero-step handler resolves directly without
leaving a target. Shulker remains a no-handler snap family. An equal active
target does not restart interpolation.

### Client-level ticking and collision

Ticking membership is queried from current level/chunk ownership at packet
application time. Player stays always ticking. Loaded/unloaded edge tests prove
that the same remote entity interpolates or snaps as Vanilla requires without a
parallel membership table.

Interpolation target carry uses the existing canonical collision query and the
entity's effective bounds. Only a collision-free position delta moves the
target; yaw and pitch carry regardless. Missing or unavailable collision data
must take the explicitly tested Vanilla-equivalent conservative path, not
silently disable the entire handler.

### Local authority, passengers, and special families

The local player is locally authoritative. A root vehicle is locally
authoritative only while the local player is its exact controlling passenger;
ordinary riding and indirect passenger relationships do not broaden authority.
Passenger reposition/reset and serverbound correction ordering are emitted as a
narrow world report consumed by native before publication.

Generated ancestry classifies generic handlers and explicit no-handler or
special behavior. Minecart track interpolation, display interpolation metadata,
hanging-entity anchors, painting bounds, multipart dragon render roots, and
projectile/fishing-hook consumers retain their dedicated owners. They may share
canonical endpoints, but no special render owner may become an independently
mutable transform path.

## Baseline and Required Tests

The phase-1 baseline records all 157 ledger rows as `partial`, one shared
`generic_transform` evidence entry, and these known implementation boundaries:

- position sync snaps every nonlocal entity instead of consulting level ticking;
- teleport always cancels/snap-applies the transform;
- generic interpolation target carry is not collision-gated;
- model candidate extraction ignores its partial-tick argument;
- generic `EntityTransform` has no old position/body-rotation endpoints;
- head rotation has no independent three-step lerp owner.

Focused tests must cover, at minimum:

- all packet forms, authority branches, exact target equality, zero/no-handler
  policies, 4096 distance boundary, and loaded/unloaded ticking membership;
- absolute and every relative teleport axis/rotation/delta combination with and
  without an active target;
- collision-free and collision-blocked self-motion carry;
- old/current endpoint initialization, tick advancement, snap reset, and
  partial-tick 0/0.5/1 projections, including wrapped body/head yaw;
- local player, controlled boat, noncontrolling passenger, indirect passenger,
  removed vehicle recovery, and ordered correction effect/publication;
- living, Shulker, Display, XP orb, fishing hook, boats, old/new minecarts,
  hanging/painting, projectile, and dragon-family special paths;
- successful same-id recreation, entity-type/id reuse, chunk load/unload,
  disconnect/respawn/dimension/reset, serialization, and old-publication
  isolation;
- render/control same-world-Arc generation pairing, effect-before-publication,
  blocked renderer, shutdown/join, and panic propagation regression guards;
- exhaustive 157-entry generated partition and coverage-ledger evidence checks.

Focused commands use the repository wrapper and `/tmp/bbb-target-main`, notably
`scripts/cargo-dev.sh test -p bbb-world <filter>`,
`scripts/cargo-dev.sh test -p bbb-native <filter>`,
`scripts/cargo-dev.sh test -p bbb-renderer <filter>`, the entity generator update
and check commands, and the coverage-ledger Python tests. Every phase also runs
`cargo fmt --all --check`, `git diff --check`, documentation lint when applicable,
and a complete diff review before its normal commit.

## Serial Phases and Gates

1. **Contract and baseline.** Land this contract and the linked unsupported
   boundary. Inventory the generated family partition, existing transform tests,
   and baseline behavior without changing runtime semantics. Run docs/fmt/diff
   checks and commit.
2. **Canonical endpoints and generic remote packets.** Add previous/current
   tick endpoints and head lerp, exact handler advancement, level-ticking and
   distance decisions, collision-gated target carry, and exact PositionSync /
   MoveEntity / Teleport behavior. Add exhaustive common/family-policy tests,
   commit buildable stages, and run a milestone full gate no later than the
   fourth engineering commit. This is the first safe stopping point.
3. **Local authority and passenger integration.** Close controlled-vehicle,
   indirect-passenger reset, removed-vehicle recovery, and ordered outbound
   correction effects without moving ownership out of `bbb-world`. Run world,
   native runtime, blocked-render, effect-order, and shutdown focused tests;
   commit and run a milestone full gate before crossing this boundary.
4. **Partial-tick renderer and special families.** Make narrow candidate and
   full render projections consume the same endpoints, then audit minecart,
   display, Shulker, hanging/painting, XP-orb, fishing-hook, projectile, dragon,
   passenger, and living head/body consumers. Preserve candidate-first
   extraction and item/model GPU residency. Run world/renderer/native scene
   tests, commit buildable stages, and run a milestone full gate.
5. **Coverage closeout.** Generate one exhaustive 157-entry transform partition,
   flip only the `transform/interpolation` cells with evidence-backed complete
   or not-applicable results, remove the matching unsupported todo, and archive
   the implementation. Run generator, ledger, focused crate, docs/fmt/diff, and
   full review checks before committing.
6. **Delivery.** At final committed HEAD run `scripts/cargo-dev.sh gate`, build
   `bbb-native` into `/tmp/bbb-target-main`, verify its timestamp is newer than
   the delivered commit, confirm a real Weston process and socket, run the
   explicit `scripts/playable-gate.sh live` with the fixed Vanilla 26.1 server,
   and then run the policy `auto` gate. Only the repository's live smoke scope is
   claimed.

Every phase uses normal commits only; no amend, rebase, reset, or history rewrite.
Concurrency/core-publication changes force a gate at their committed boundary
even when fewer than four commits have elapsed. The final delivered HEAD always
receives a fresh full gate.

## Failure Localization and Safe Stop

Packet/base and generic-handler changes land before local vehicle side effects;
renderer projection lands only after canonical endpoints are stable; coverage
changes land last. A focused failure is fixed forward in the phase that owns it.
If a milestone full gate fails and focused diagnosis cannot identify the phase,
`scripts/cargo-dev.sh batch-gate <last-gated>..HEAD` may localize the introducing
commit, but history is never rewritten.

The first safe stop is a committed, fully gated generic remote packet and
endpoint foundation. Later safe stops require a committed full gate at the
local-authority or renderer milestone. A real blocker is an unresolvable Vanilla
semantic dependency or an ownership change that invalidates this frozen
contract; size, difficult tests, or a live-gate failure with actionable evidence
are not blockers. Any live failure records its artifact directory, distinguishes
environment/client/control/assertion state, and is rerun from a current-HEAD
binary after a forward fix.
