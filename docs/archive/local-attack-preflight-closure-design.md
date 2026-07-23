# Local attack preflight packet coverage closure

Status: closed and archived after the applicable packet-coverage cells passed.

## Goal and completion contract

Close the remaining state-guard and test coverage cells for
`ServerboundAttackPacket` by reproducing the Vanilla 26.1 local attack
preflight, cadence resets, and carried-item send ordering. A left attack press
must observe `missTime`, boat-control hands-busy state, spectator routing,
selected-item feature availability, `minimum_attack_charge`, piercing-weapon
dispatch, custom attack range, and the existing use-item gate in Vanilla order.
An accepted entity attack must ensure the selected hotbar slot reaches the
server before the attack packet, reset both local attack-strength clocks, and
then retain the existing local main-hand swing ordering.

Completion means the source-derived default `minimum_attack_charge` value for
every fixed Vanilla item is available in the world item profile; an ItemStack
patch can add, override, or remove that value; local player ticks reproduce
Vanilla's attack-strength and item-swap clocks plus main-hand identity reset;
misses impose the ten-tick non-Creative delay and reset attack strength;
left-button release clears that delay; boat directional input blocks attack;
disabled or insufficiently charged stacks emit no action or swing; every
connection deduplicates ordinary hotbar selection sends while still repairing
selection before an attack; reconnect starts with no assumed server selection;
the packet coverage row is complete in every applicable dimension; the
remaining unsupported-feature entry names only target-specific client
prediction and presentation; and the full gate plus applicable playable smoke
pass at the milestone head.

## Scope and non-goals

- `bbb-pack` extracts the prototype `minimum_attack_charge` float from the
  local Vanilla item declarations, including the value supplied by
  `Item.Properties.spear`.
- `bbb-item-model` projects those source-derived values by protocol item id.
- `bbb-world` owns prototype values, ItemStack patch precedence, the local
  player's two attack-strength clocks and last-main-hand snapshot, actual
  modifier-folded attack-speed lookup, creative/spectator facts, and narrow
  attack preflight/reset queries.
- `bbb-native` owns `Minecraft.missTime`, current directional input, target
  selection, and exact input branch ordering. It passes the selected hotbar
  slot with an accepted attack command.
- `bbb-net` owns the serverbound carried-slot observation for one
  `RawConnection`. Successful explicit selection updates that observation;
  attack sending first repairs a missing or different observation and then
  emits the attack. A new transport begins with no observation.

This slice does not predict damage, knockback, target death, inventory damage,
enchantment side effects, critical-hit or enchanted-hit particles, or the
special `Entity.hurtClient` / redirectable-projectile branches inside
`Player.attack`. Those target-specific presentation paths remain explicit
follow-up work and server damage/combat outcomes remain authoritative. This
slice does not change entity-use ordering, block-destruction authority,
right-click prediction, screen UI, or protocol wire formats.

## Vanilla and integration contract

`Minecraft.startAttack` first returns while `missTime > 0`, then rejects a
null result with a ten-tick miss delay when `MultiPlayerGameMode.hasMissTime`.
It rejects `LocalPlayer.isHandsBusy`, lets spectator entity hits call
`spectate`, requires the held stack to be enabled by the level feature set,
and rejects `Player.cannotAttackWithItem(stack, 0)`. A piercing weapon sends
the stab action and swings immediately. An entity hit calls
`MultiPlayerGameMode.attack` only when its optional `AttackRange` accepts the
hit. Block and miss fallthrough retain the existing destroy behavior; a miss
sets the non-Creative delay and resets attack strength before the common swing.
`continueAttack(false)` clears `missTime`, while each gameplay-input
`Minecraft.tick` decrements a positive delay once.

All fixed 26.1 item declarations use the Vanilla feature set; an empty stack is
enabled, while a non-empty fixed stack requires `minecraft:vanilla` in the
server-synchronized enabled feature set. Unknown or invalid item ids are
rejected rather than treated as an enabled Vanilla prototype.
`Item.Properties.spear` supplies
`DataComponents.MINIMUM_ATTACK_CHARGE = 1.0F`; other stacks fall back to
`0.0F` unless their prototype or patch supplies the component. A removed
component wins over the prototype, and an added patch value wins over it.

`Player.tick` increments `attackStrengthTicker` and `itemSwapTicker`, compares
the complete prior/current main-hand stacks with `ItemStack.matches`, resets
both clocks only when `ItemStack.isSameItem` changes, and copies every changed
stack as the next comparison source. `cannotAttackWithItem` computes
`requiredStrength > 0 && attackStrengthTicker /
(20 / actualAttackSpeed) < requiredStrength`. bbb saturates integer clocks and
treats a non-finite or non-positive attack speed as uncharged for a positive
requirement instead of allowing invalid synchronized data to bypass the gate.
`resetAttackStrengthTicker` sets both clocks to zero.

`LocalPlayer.rideTick` marks hands busy only when the locally controlled
first-passenger root vehicle is an `AbstractBoat` and any left, right, forward,
or backward input is held. `MultiPlayerGameMode.attack` calls
`ensureHasSentCarriedItem`, sends `ServerboundAttackPacket`, invokes the local
presentation branch, then resets attack strength. Because target-specific
presentation is outside this slice, bbb freezes the observable supported
ordering as carried-slot repair, attack packet, canonical strength reset, and
the existing common main-hand swing. `RawConnection` updates its carried-slot
observation only after a successful send, so backpressure or I/O failure cannot
claim an unsent selection.

Authoritative local sources are
`net/minecraft/client/Minecraft.java#startAttack`,
`net/minecraft/client/Minecraft.java#continueAttack`,
`net/minecraft/client/multiplayer/MultiPlayerGameMode.java#attack`,
`#ensureHasSentCarriedItem`, and `#hasMissTime`,
`net/minecraft/client/player/LocalPlayer.java#rideTick`,
`net/minecraft/world/entity/player/Player.java#tick`,
`#cannotAttackWithItem`, `#getCurrentItemAttackStrengthDelay`, and
`#resetAttackStrengthTicker`,
`net/minecraft/world/item/ItemStack.java#isItemEnabled`, and
`net/minecraft/world/item/Item.java#Properties.spear` under the Vanilla 26.1
source tree.

## Serial stages and verification

1. Extract and project default minimum-attack-charge values through
   `bbb-pack` and `bbb-item-model`; add world prototype/patch queries, actual
   local attack-speed lookup, local attack clock state, exact tick/item-swap
   transitions, feature/creative facts, and reset semantics. Focused tests
   cover spear/default extraction, runtime projection, add/override/remove
   patches, empty/invalid/disabled stacks, all charge boundaries, stack
   count/component versus item-identity changes, bulk ticks, level/session
   reset, and serialized snapshots with
   `scripts/cargo-dev.sh test -p bbb-pack minimum_attack_charge`,
   `scripts/cargo-dev.sh test -p bbb-item-model minimum_attack_charge`, and
   `scripts/cargo-dev.sh test -p bbb-world local_attack`.
2. Add the selected-slot attack command and connection-owned carried-slot
   observation; apply native miss-time, hands-busy, feature, charge, miss, and
   accepted-attack reset ordering. Focused tests prove explicit selection
   deduplication, reconnect reset, same/different slot attack sends, I/O
   ordering, every native preflight rejection, Creative versus Survival miss
   timing, piercing and spectator precedence, `Attack -> Swing` command order,
   and strength reset with
   `scripts/cargo-dev.sh test -p bbb-net attack_entity` and
   `scripts/cargo-dev.sh test -p bbb-native local_attack`.
3. Mark the serverbound packet row complete, replace its reverse partial guard,
   regenerate and check the ledger, narrow the exact unsupported attack entry,
   archive this contract, and append the milestone index. Run
   `scripts/cargo-dev.sh test -p bbb-protocol packet_coverage`, review the
   integrated diff, then run `BBB_GATE_FULL=1 scripts/cargo-dev.sh gate`,
   `scripts/playable-gate.sh live`, and `scripts/playable-gate.sh auto`. A
   missing configured Java executable is recorded as live-environment
   unavailable rather than a product failure; the checked-in auto fixture must
   pass.

The fully gated `609a5e422` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Pack/world failures stop before net/native work; net/native
failures stop before coverage closeout. The final closeout is the fourth commit
including this contract and therefore forces a full gate. A failed full gate is
localized from the pre-start checkpoint only when focused tests do not identify
it. Failures are fixed forward without rewriting history.

Closeout completed every applicable serverbound attack packet-coverage cell
after source-derived item charge, canonical strength-clock, native preflight,
and carried-slot-before-attack regressions passed.
