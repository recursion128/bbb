# Player-action packet coverage closure

## Goal and completion contract

Close the remaining partial coverage cells for
`ServerboundPlayerActionPacket`. The native input path must preserve Vanilla's
prediction-before-send ordering for block destruction, run the complete
client-observable `BlockState.attack` family, emit one correctly shaped terrain
hit particle and local main-hand swing for every successful held destroy tick,
and reconcile every predicted state through the ordered block-update/ack path.

Completion means the already generated constructor-level infested destroy
profiles and the exact standing-eye fluid probe are explicitly regression
locked and their stale ledger entries removed; redstone ore and dragon egg run
their client callbacks while note block remains server-only; repeated
predictions at one position retain the first known server state while advancing
their concluding sequence; START, ABORT and STOP use Vanilla direction,
prediction and command ordering; all eight action kinds have focused native,
net or protocol evidence; the packet row has all five coverage dimensions
`complete`; the finished block-destroy TODOs are removed; and the full plus
playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-pack` derives the `canOcclude` bit used by redstone-ore face particles
  from the checked-in `Blocks.java` expression catalog. It does not introduce a
  hand-written block membership table.
- `bbb-world` owns destroy speed, active destroy state, generic predicted block
  replacement, update/ack reconciliation, redstone-ore lit prediction, and the
  typed local block-attack effect request.
- `bbb-native` owns input and outbound command ordering, local swing triggering,
  outline-bound hit sampling, redstone/portal particle resolution, and particle
  batch submission. The renderer's existing particle command path remains the
  only particle execution owner.
- Existing protocol and net encoders remain authoritative. This slice adds
  coverage regressions rather than a second encoder or command path.

This slice does not complete entity attack or entity-use parity, item feature
gates, hands-busy/miss-time behavior, attack-strength reset, server combat or
drop outcomes, server neighbour/redstone simulation, or redstone-ore random
unlighting. It does not predict the off-hand swap: Vanilla sends that action and
waits for authoritative inventory synchronization. It does not add an in-game
configuration surface.

## Vanilla, wire, and integration contract

`ServerboundPlayerActionPacket.STREAM_CODEC` writes the action enum, packed
position, direction byte and prediction VarInt. Its eight actions are START,
ABORT and STOP destroy, drop stack, drop one, release use, swap offhand and
STAB. `ServerGamePacketListenerImpl.handlePlayerAction` forwards only the three
destroy actions to `handleBlockBreakAction` and advances the block-change ack;
drop, swap, release and STAB remain server-authoritative apart from Vanilla's
local drop and use-state predictions. Swap has no client inventory prediction.

`MultiPlayerGameMode.startDestroyBlock` opens a prediction before any local
mutation. On a new survival target it aborts the old position with the new hit
direction, invokes `state.attack` once while progress is zero, performs an
immediate destroy or initializes the target/item/progress state, then sends
START with that prediction sequence. Creative prediction destroys first, sends
START second and installs a five-tick delay. `stopDestroyBlock` sends ABORT with
`Direction.DOWN` and clears local progress. `continueDestroyBlock` first makes
the selected item current on the server, advances progress, plays the hit sound
when the pre-increment destroy tick is divisible by four, and on completion
opens a new prediction, destroys locally, sends STOP, resets progress/ticks and
installs the five-tick delay. `Minecraft.continueAttack` then adds the hit
particle and calls `LocalPlayer.swing` only when continuation returned true;
because the final prediction has already replaced the block with air, the final
particle lookup emits nothing while the swing packet is still sent.

`BlockStatePredictionHandler.retainKnownServerState` keeps one entry per block
position. A later prediction at the same position updates only its concluding
sequence; it retains the first server state and player position. Server block
updates replace the retained server state without changing the visible
prediction. `ClientboundBlockChangedAckPacket` ends every prediction through
the acknowledged sequence and synchronizes the latest retained server state.
Thus bbb must mutate and record before enqueueing START/STOP, must not replace
the first retained state on a repeated prediction, and must continue to defer
accepted single/section updates until the ack.

Vanilla 26.1 has exactly three `BlockBehaviour.attack` overrides. `NoteBlock`
does work only on the server, so the client callback is empty. `RedStoneOreBlock`
spawns red dust on each adjacent face whose neighbour is not `isSolidRender`,
then predicts `lit=true` when needed. Face iteration is Direction enum order;
axis-fixed coordinates use `0.5 + 0.5625 * step`, while the other two axes each
consume `level.random.nextFloat()`. `isSolidRender` is the full occlusion-shape
bit: generated `canOcclude` plus the native shape projection and the four
Vanilla `getOcclusionShape` override families determine it without texture
heuristics. `DragonEggBlock` makes at most 1,000 random candidate probes using
the exact paired `nextInt(16)`, paired `nextInt(8)`, paired `nextInt(16)` order;
the first in-height, in-border air position above a non-air block emits 128
portal particles from the original position toward the candidate and does not
move either block on the client.

`ClientLevel.addBreakingBlockEffect` rejects only invisible render shape or
`shouldSpawnTerrainParticles=false`, samples the bounds of `state.getShape`
with three `level.random.nextDouble()` draws, pins the selected face 0.1 blocks
outside those bounds, and constructs a terrain particle at zero auxiliary
velocity with power `0.2` and scale `0.6`. The existing native block-outline
projection supplies those exact shape bounds and the existing terrain-particle
texture/tint path supplies the block-state sprite and biome colour.

`Player.getDestroySpeed` applies mining efficiency only above base speed one,
then Haste/Conduit, Mining Fatigue, block-break-speed, submerged-mining-speed
when `EntityFluidInteraction` reports the canonical eye inside water, and the
airborne divisor. The current world fluid projection already uses pose-specific
eye height and the same inclusive fluid-bottom/top test. `InfestedBlock` and
`InfestedRotatedPillarBlock` likewise already derive half the host block's
constructor destroy time from the checked-in Java expression catalog; these
facts need regression/ledger closeout, not a parallel implementation.

Authoritative local sources are
`net/minecraft/network/protocol/game/ServerboundPlayerActionPacket.java`,
`net/minecraft/server/network/ServerGamePacketListenerImpl.java`,
`net/minecraft/client/Minecraft.java`,
`net/minecraft/client/multiplayer/MultiPlayerGameMode.java`,
`ClientLevel.java`,
`net/minecraft/client/multiplayer/prediction/BlockStatePredictionHandler.java`,
`net/minecraft/world/entity/player/Player.java`,
`net/minecraft/world/entity/EntityFluidInteraction.java`,
`net/minecraft/world/level/block/InfestedBlock.java`,
`InfestedRotatedPillarBlock.java`, `NoteBlock.java`, `RedStoneOreBlock.java`,
`DragonEggBlock.java`, and `BlockBehaviour.java` under the Vanilla 26.1 source
tree.

## Serial stages and verification

1. Derive block `canOcclude` profiles from the checked-in Java expression IR,
   regression-lock all 1,168 declarations plus copied/helper/no-occlusion cases,
   and expose the catalog to the native particle runtime. Focused tests:
   `bbb-pack` occlusion-profile catalog tests and existing destroy-profile tests.
2. Generalize local predicted replacement so repeated same-position predictions
   retain the first server state, add typed block-attack dispatch and
   redstone-ore lit prediction, and lock the existing infested and standing-eye
   destroy-speed facts. Focused tests: `bbb-world` prediction/update/ack,
   attack-dispatch, destroy-profile and fluid/destroy-speed tests.
3. Reorder START/STOP around prediction, add held-destroy swing and exact hit,
   redstone and dragon-egg particle batches, and submit pending input effects in
   the simulation pump. Focused tests: `bbb-native` mouse ordering and particle
   resolver/runtime tests plus existing `bbb-net` player-action command tests.
   This cross-boundary prediction/input/particle milestone gets a full gate.
4. Add the eight-action focused coverage evidence, mark the packet row complete,
   remove the four finished block-destroy TODOs, append the milestone index,
   review the integrated diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `909fe9896` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Catalog failures stop before world prediction changes;
prediction/ack failures stop before native input integration; particle or
ordering failures stop before coverage closeout. Failures are fixed forward
without rewriting history.
