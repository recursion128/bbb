# Local Player Travel Ordering Design

Engineering contract for aligning the local player's non-fluid, non-flying
travel pipeline with the vanilla 26.1 per-tick `deltaMovement` ordering. The
production input cadence is already fixed-tick
(`bbb-native/src/input/movement.rs` drives
`WorldStore::advance_local_player_input` with 0.05s ticks), so exact per-tick
vanilla semantics are attainable without re-architecting the integrator.

## Vanilla 26.1 contract (source anchors)

Tick order, from `LivingEntity.aiStep`:

1. `noJumpDelay` decrements at the top of every tick
   (`LivingEntity.java#aiStep`).
2. Input scaling: `LocalPlayer.modifyInput` multiplies the raw input by
   `0.98F`, then by the item-use multiplier while using an item off-vehicle,
   then by the `SNEAKING_SPEED` attribute while moving slowly
   (`LocalPlayer.java#modifyInput`).
3. Jump gate: while the jump input is held, a ground jump fires only when
   `noJumpDelay == 0`, then sets `noJumpDelay = 10`; releasing the input
   resets it to `0` immediately. Liquid jumps (`jumpInLiquid`) are not gated.
   `jumpFromGround` sets `deltaMovement.y = max(jumpPower, y)` where
   `jumpPower = JUMP_STRENGTH attribute * blockJumpFactor + jumpBoost`, and a
   sprinting jump adds `(-sin(yRot) * 0.2, 0, cos(yRot) * 0.2)`
   (`LivingEntity.java#jumpFromGround`, `#getJumpPower`).
4. `travel` → `travelInAir` (`LivingEntity.java#travelInAir`):
   - `blockFriction` = friction of the block below when on ground, else `1.0`;
     `friction = blockFriction * 0.91`.
   - `handleRelativeFrictionAndCalculateMovement`:
     `moveRelative(getFrictionInfluencedSpeed(blockFriction), input)` **adds**
     the yaw-rotated input vector (normalized only when its length exceeds 1)
     times speed to `deltaMovement`; speed is
     `getSpeed() * 0.21600002 / blockFriction³` on ground, else
     `getFlyingSpeed()` (player air control: `0.025999999` sprinting, `0.02`
     otherwise; creative flight uses `abilities.getFlyingSpeed()` ×2 when
     sprinting). Then `handleOnClimbable` clamps the delta (±0.15 horizontal,
     y ≥ −0.15, sneak-on-ladder suppression), then `move(SELF, delta)`.
   - Post-move: if `(horizontalCollision || jumping)` and on a climbable or
     walkable powder snow, the y component becomes `0.2`.
   - Levitation moves y toward `0.05 * (amplifier + 1)` by 20% per tick;
     otherwise y loses `getEffectiveGravity()` (gravity attribute, default
     `0.08`; slow falling caps it at `0.01` while falling).
   - Final friction multiply: `(x * friction, y * 0.98, z * friction)`
     (vertical `0.98` for non-`FlyingAnimal`).
5. `Entity.move` ordering (`Entity.java#move`): consume stuck-speed multiplier
   (zeroing the stored delta) → `maybeBackOffFromEdge` → `collide` →
   fall-damage-resetting clip for ≥1-block moves → `setPos` → collision flags
   via `Mth.equal` → `setOnGroundWithMovement` → zero the collided horizontal
   delta axes (vertical is **not** zeroed here) → when `delta.y != movement.y`
   the supporting block's `updateEntityMovementAfterFallOn` runs (the base
   `Block` impl zeroes `deltaMovement.y`; slime negates it, beds scale it) →
   movement emission/sounds → multiply the horizontal delta by
   `getBlockSpeedFactor()`.
6. `Player.travel` wrapper (`Player.java#travel`): creative flight preserves
   the pre-travel `deltaMovement.y` and rescales it by `0.6` after
   `super.travel`; swimming steers y toward the look angle before travel.

## Current bbb model and deltas

`advance_local_player_physics_step`
(`bbb-world/src/client/local_player_movement.rs`) treats input as a direct
per-tick displacement (`input × speed × dt`) and rebuilds `delta_movement`
from the clipped displacement (`movement / tick_span`), so horizontal
momentum does not persist between ticks and ground friction/ice sliding do
not exist. Jump power, sprint jump impulse, climbable clamp values, stuck
multiplier consumption, edge back-off, block speed/jump factors, and the
airborne vertical chain already have vanilla-aligned owners; they must be
re-plumbed into the ordering above rather than reimplemented.

## Stages

Each stage is independently valuable, lands with focused tests plus a full
gate (core physics is high-risk), and is a safe stop point.

- **S1 — jump cadence (landed with this contract)**: add the `noJumpDelay`
  cooldown chain (10-tick held-jump lockout, instant reset on release,
  decrement-then-gate ordering) to the pose state and ground jump gate.
- **S2 — input scale chain**: consolidate the `0.98 × item-use ×
  SNEAKING_SPEED` multiplier ordering into one vanilla-shaped input pipeline
  and pin each factor with tests.
- **S3 — core travel chain**: replace the displacement model with the
  accumulate → clamp → move → impulse → gravity → friction-multiply ordering
  of `travelInAir`/`Entity.move` above, including the base-block fall-on
  callback owning the vertical zeroing (slime bounce becomes its override)
  and the post-move block speed factor multiply. Expected observable changes
  (ground momentum, ice slide, air control) must land with fixture
  expectations derived from vanilla formulas, not from the old model.
- **S4 — player wrappers**: creative-flight y×0.6 preservation and the
  sprint air-control constant; verify the swimming surface steer against the
  fluid branch owner.
- **S5 — closure**: ledger/matrix updates, playable-gate fixture plus live
  movement evidence, archive this contract.

Non-goals: fluid and fall-flying travel branches (separate vanilla methods
with their own owners), ridden/vehicle travel, and server-authoritative
position rollback semantics, which stay unchanged.
