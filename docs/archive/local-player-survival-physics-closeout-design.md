# Local Player Survival Physics Closeout Design

Engineering contract for closing the remaining client-observable Vanilla 26.1
survival-movement behavior after fixed-tick travel ordering and the shared
block/entity/world-border collision owner landed.

## Goal and completion standard

The unmounted local player must reproduce the remaining `LocalPlayer.aiStep`
and `Entity.move` survival-physics behavior that changes predicted movement,
sprint state, or local audio:

- four-corner suffocation escape against blocks whose live state is
  `isSuffocating`;
- optional auto-jump, enabled by default and configurable only at process
  startup, using Vanilla `getCollisions`' shared entity-then-block collision
  set (which deliberately excludes the separately queried world border);
- `minorHorizontalCollision` classification and run-sprint cancellation on a
  non-minor collision;
- the up-to-eight-block fall-distance-resetting ray through the synchronized
  `minecraft:fall_damage_resetting` block tag and water;
- bed vertical bounce and the Honey Block landing slide sound.

Completion requires exact source-derived block facts/memberships, focused
world/native tests for every branch and ordering edge above, removal of the
broad survival-physics TODO, an archived contract/index entry, a clean reviewed
diff, and a fully gated final commit.

## Vanilla 26.1 anchors

- `LocalPlayer.aiStep`, `moveTowardsClosestSpace`, `suffocatesAt`,
  `move`, `updateAutoJump`, `canAutoJump`,
  `isHorizontalCollisionMinor`, `shouldStopRunSprinting`
- `CollisionGetter.collidesWithSuffocatingBlock`,
  `CollisionGetter.getCollisions`, `BlockCollisions`
- `Entity.move`
- `ClipContext.Block.FALLDAMAGE_RESETTING`,
  `ClipContext.Fluid.WATER`
- `BlockBehaviour.BlockStateBase.isSuffocating`
- `BedBlock.updateEntityMovementAfterFallOn`
- `HoneyBlock.fallOn`
- `ClientLevel.getPushableEntities`

The local source and data roots are
`~/Work/mc-code/sources/26.1/net/minecraft/` and
`~/Work/mc-code/sources/26.1/data/minecraft/tags/block/`.

## Frozen behavior and ownership

- `bbb-protocol::block_state_types` owns the runtime-generated per-state
  `isSuffocating` fact beside its existing support facts. Dynamic piston and
  shulker states are evaluated from live properties where Vanilla's predicate
  is state-dependent.
- `bbb-world::terrain` owns the generated Vanilla fallback membership for
  `minecraft:fall_damage_resetting`; a synchronized block tag is authoritative
  when present.
- `bbb-world::client::local_player_collision` remains the only block/entity/
  border shape collector and ray geometry owner. Movement calls narrow
  suffocation, prospective entity-then-block collision-box, and fall-reset ray
  queries instead of duplicating shapes; auto-jump does not add the border to
  `CollisionGetter.getCollisions`.
- `bbb-world::client::local_player_movement` owns the per-tick order:
  consume/force an armed auto-jump input, update pose, apply four escape probes,
  travel and `Entity.move`, update auto-jump from the actual horizontal move,
  then apply post-travel block effects.
- Auto-jump enablement and its private one-tick timer live with
  `LocalPlayerState`; startup sets enablement from a `--client-auto-jump`
  boolean whose Vanilla-compatible default is `true`. There is no in-game
  configuration surface.
- `LocalPlayerPoseState` carries `minor_horizontal_collision` next to
  `horizontal_collision`. `bbb-native` evaluates the previous movement result
  before each unmounted client tick, clears its sprint latch on a major
  collision, and emits the existing stop-sprint command exactly once.
- Honey landing uses the existing ordered entity-client movement effect queue
  and deterministic sound-seed owner. Bed bounce stays in the movement tail.
- Missing/unloaded blocks remain non-colliding. Missing synchronized
  `fall_damage_resetting` tags use the generated Vanilla fallback; a present
  server tag, including an explicitly empty one, replaces that fallback.

## Explicit Vanilla adjudications

- `LivingEntity.pushEntities` is not a local-player client gap:
  `ClientLevel.getPushableEntities(pusher, bounds)` returns only the local
  player and only when `player != pusher`; therefore the local player's own
  call is empty. Remote entities pushing the local player remain owned by the
  client entity tick path.
- End/nether portal additions in
  `ClipContext.Block.FALLDAMAGE_RESETTING` do not broaden this client slice:
  end portal/gateway are player hits and are included, while the zero-delay
  nether-portal rule is a `ServerLevel`-only branch.
- Sprint-swim camera, animation, and presentation stay in their separate
  ledger item. Server-authoritative damage, exhaustion, and correction remain
  unchanged.

## Serial stages, tests, and gates

Each stage is a safe stopping point. Core movement, generated data, and scripts
make this a high-risk engineering slice; every stage commit receives a full
gate before the next stage starts.

1. **Generated facts and fallback membership**: extend the checked-in Vanilla
   runtime probe, generator parser/tests, protocol fact projection, and block
   membership catalog for `isSuffocating` and
   `fall_damage_resetting`.
   Focused tests: generator unit tests, protocol block-state tests, and terrain
   membership tests.
2. **Escape and auto-jump**: add the local-player private timer/config state,
   collision-owner queries, exact four-corner escape and auto-jump ordering,
   plus block/entity/border-exclusion/ceiling/disabled-option tests.
   Focused command:
   `scripts/cargo-dev.sh test -p bbb-world local_player_`.
3. **Move-tail completion**: add minor-collision classification, fall-reset
   ray, bed bounce, Honey Block sound effect/mapping, and focused ordinary/
   fluid/fall-flying tests.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-world local_player_` and
   `scripts/cargo-dev.sh test -p bbb-world entity_client_movement_sound`.
4. **Native lifecycle and closeout**: wire the startup-only auto-jump option,
   cancel sprint at the Vanilla tick boundary, test catch-up and command
   ordering, remove the broad ledger TODO, archive this contract, update the
   milestone index, run the final audit, review, commit, and gate.
   Focused commands:
   `scripts/cargo-dev.sh test -p bbb-native startup` and
   `scripts/cargo-dev.sh test -p bbb-native input::movement`.

Any stage failure is fixed forward on the current stage. No unrelated slice
starts until this contract is closed or a concrete external blocker is
recorded.
