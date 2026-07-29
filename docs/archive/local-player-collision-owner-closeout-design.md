# Local Player Collision Owner Closeout Design

Engineering contract for replacing the local player's remaining block-only
movement probes with the same Vanilla-shaped block, entity, and world-border
collision owner already used by locally authoritative vehicles and client
entity movement.

## Goal and completion standard

Every unmounted local-player `Entity.move(MoverType.SELF, ...)` path — ordinary
air/ground travel, fluid travel, fall flying, and the narrow grounded Riptide
move — must resolve one collision set containing:

- live `BlockState.getCollisionShape` AABBs with the local
  `EntityCollisionContext`;
- entities accepted by `EntityGetter.getEntityCollisions`;
- the world-border shape when `WorldBorder.isInsideCloseToBorder` exposes it.

The same collision set must govern body-pose fit checks and
`Player.maybeBackOffFromEdge` support probes. Spectator movement must follow
`Player.tick` plus `Entity.move`'s `noPhysics` branch: clear grounded state,
skip collision/backoff/block effects, and preserve unrestricted requested
movement.

Completion requires focused tests for entity, border, same-vehicle,
pose-fit/edge support, spectator, ordinary/fluid/fall-flying ordering, and the
exact Vanilla horizontal collision flag tolerance. The production movement
path must no longer own a second binary-search block clipper.

## Vanilla 26.1 anchors

- `Entity.move`, `Entity.collide`, `Entity.collideBoundingBox`,
  `Entity.collectColliders`
- `EntityGetter.getEntityCollisions`
- `Player.tick`, `Player.maybeBackOffFromEdge`
- `Player.updatePlayerPose`, `Player.canPlayerFitWithinBlocksAndEntitiesWhen`
- `LivingEntity.travelInAir`, `travelInFluid`, `travelFallFlying`
- `LocalPlayer.move`

The local sources under `~/Work/mc-code/sources/26.1/` fix collision ordering,
step-height candidates, entity/same-vehicle filters, world-border inclusion,
and the `Mth.equal` horizontal collision threshold.

## Ownership and integration contract

- `bbb-world::client::local_player_collision` remains the sole AABB collector
  and axis clipper. It exposes narrow local-player movement and bounds-free
  queries backed by its existing generated entity collidability and live block
  context.
- `bbb-world::client::local_player_movement` owns travel ordering only. It
  snapshots one bounded entity-collider batch per input integration and passes
  it through ordinary, fluid, fall-flying, pose, and edge probes.
- No protocol, net, native-input, renderer, or public control API changes are
  part of this engineering slice.
- Missing/unloaded block sections retain the existing Vanilla client behavior:
  unloaded block collisions are absent. Entity and border colliders remain
  independently available.

## Non-goals

- `LocalPlayer.moveTowardsClosestSpace` suffocation escape
- optional auto-jump
- post-travel `LivingEntity.pushEntities`
- sprint-swim camera, animation, and presentation
- server-authoritative damage, position correction, and vehicle movement

These remain candidates under the broader survival-physics and presentation
ledger entries.

## Stages and gates

1. Refactor the collision owner around one context-aware local-authority
   helper, then add local-player movement and bounds-free facades.
2. Thread one entity-collider snapshot through every local-player movement
   branch; remove the block-only clipper and align collision flags.
3. Add focused semantic tests, update the gap ledger and archive, review the
   integrated diff, run per-commit checks, commit, and run the full gate because
   locally authoritative movement is P0-sensitive.

Focused command:
`scripts/cargo-dev.sh test -p bbb-world local_player_`.
