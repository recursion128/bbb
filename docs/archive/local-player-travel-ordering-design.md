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

Before this contract, `advance_local_player_physics_step`
(`bbb-world/src/client/local_player_movement.rs`) treated input as a direct
per-tick displacement (`input × speed × dt`) and rebuilt `delta_movement`
from the clipped displacement (`movement / tick_span`), so horizontal
momentum did not persist between ticks and ground friction/ice sliding did
not exist. With S1–S4 landed, the ground/air/creative-flight paths run the
vanilla ordering above on the carried delta. Remaining deltas: the collision
epsilon stays at `1.0e-7` (vs `Mth.equal`'s `1.0e-5`), and the fluid branch
still consumes the pre-rewrite normalized direction input pending its
owner's `modifyInput` alignment.

## Stages

Each stage is independently valuable, lands with focused tests plus a full
gate (core physics is high-risk), and is a safe stop point.

- **S1 — jump cadence (landed)**: the `noJumpDelay` cooldown chain (10-tick
  held-jump lockout, instant reset on release, decrement-then-gate ordering)
  in the pose state and ground jump gate.
- **S2 — input scale chain (landed)**: `local_player_modified_input` owns the
  `0.98 × item-use × SNEAKING_SPEED` ordering plus the unit-square
  projection; the fluid and creative-flight branches keep the legacy
  normalized direction input until their own stages align them.
- **S3 — core travel chain (landed)**: the ground/air path runs the
  accumulate → climbable clamp → move → fall-on callback (base zeroes the
  vertical delta, slime reflects) → block speed factor → wall-climb impulse
  → gravity → friction-multiply ordering. Grounded rest carries the vanilla
  `-0.08 * 0.98` vertical delta, step-up eligibility follows
  `Entity.collide` (grounded or landing, not `requested_y == 0`), and the
  movement fixtures encode first-tick acceleration plus carried momentum
  derived from the vanilla formulas. Known deviation: collision epsilon
  stays at the repo's `1.0e-7` instead of `Mth.equal`'s `1.0e-5`.
- **S4 — player wrappers (landed)**: creative flight runs the shared chain
  instead of a separate displacement path. The vertical fly input
  (±`abilities.getFlyingSpeed() * 3`) is added before the chain per
  `LocalPlayer.aiStep`, the fall distance resets every flying tick per
  `Player.aiStep`, airborne acceleration uses `Player.getFlyingSpeed`
  (abilities speed, ×2 sprinting), and the vertical delta is rebuilt from the
  pre-travel snapshot ×0.6 per `Player.travel`, discarding the
  gravity/collision tail exactly as vanilla does. Flying pins
  `Player.onClimbable` to false (no clamp, no wall-climb impulse) and
  `Player.getBlockSpeedFactor` to 1.0; the edge back-off already carried its
  own flying gate. Flying-while-grounded keeps the ground friction/speed
  formula, matching `getFrictionInfluencedSpeed`'s `onGround` key. The
  swimming surface steer stays with the fluid branch owner
  (`isSwimming()` is false while flying).
- **S5 — closure (landed)**: the unsupported-features movement queue now
  carries only the fluid-branch `modifyInput` follow-up, the playable-gate
  fixture smoke and the live gate (whose movement scenario drives the real
  input path and asserts ≥1 block displacement with zero server position
  corrections) passed on the rewritten chain, and this contract is archived.

Non-goals: fluid and fall-flying travel branches (separate vanilla methods
with their own owners), ridden/vehicle travel, and server-authoritative
position rollback semantics, which stay unchanged.
