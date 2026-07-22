# Command block editor packet closure

## Goal and completion contract

Close all five coverage cells for
`play/serverbound/SERVERBOUND_SET_COMMAND_BLOCK` with one native vertical path.
A permitted local player who uses a loaded command block opens the in-game command
block editor while the ordinary `UseItemOn` request still reaches the server. The
server's matching `ClientboundBlockEntityDataPacket` enables and refreshes the
editor from typed command-block state. Done sends one typed
`ServerboundSetCommandBlockPacket`; Cancel sends nothing. The server remains
authoritative for block replacement, command execution, output, and diagnostics.

Completion means the wire encoder matches Vanilla 26.1 exactly, command text is
bounded to 32,500 UTF-16 code units before encoding, mode ordinals and flag bits
are typed, and every online driver path has an explicit command arm. World state
retains the command-block NBT fields needed by the editor without inventing a
parallel packet snapshot. The editor supports command text, caret and selection,
local/server command suggestions, previous output, track-output, mode,
conditional, automatic, Done, and Cancel. It closes when the block entity becomes
invalid or the level is replaced. Focused semantic tests, the full gate, and the
applicable playable gate pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns `CommandBlockMode`, `SetCommandBlock`, the exact encoder,
  bounds, flag packing, and wire tests in a command-block packet-family module.
- `bbb-net` owns the `NetCommand` transport arm and the unique driver send path.
- `bbb-world` owns typed command-block block-entity data, active editor identity,
  permission/block validity guards, authoritative packet refresh, and reset.
- `bbb-native` owns input focus, editing, command-suggestion interaction,
  submission, cancellation, narration handoff, and world-to-HUD projection.
- `bbb-renderer` owns the immutable HUD value and deterministic GUI geometry.

This slice does not implement server command execution, predict block replacement,
invent a local command-block-enabled setting, add configuration UI, or close the
whole clientbound BlockEntityData row. The server's existing system-chat path owns
`advMode.notAllowed` and `advMode.notEnabled` results. Minecart command blocks,
structure blocks, jigsaws, test blocks, and game-rule screens remain separate
packet/screen families. Trial-spawner `SpawnData` and synthetic entity NBT remain
outside this command-block-specific projection.

## Vanilla and integration contract

`ServerboundSetCommandBlockPacket.STREAM_CODEC` writes packed `BlockPos`, an
unbounded-default `writeUtf` string constrained by the editor's 32,500-character
maximum, `CommandBlockEntity.Mode` enum ordinal, then one byte with track-output
bit `1`, conditional bit `2`, and automatic bit `4`. Mode declaration order is
`SEQUENCE`, `AUTO`, `REDSTONE`.

`CommandBlock.useWithoutItem` opens the local editor only for a command-block
entity and `Player.canUseGameMasterBlocks`, while the server handles the same use
and sends `ClientboundBlockEntityDataPacket.create(..., saveCustomOnly)`. The
client editor initially disables controls. A matching block-entity update loads
`BaseCommandBlock` fields plus `powered`, `conditionMet`, and `auto`, refreshes the
open editor, and enables controls. Mode comes from the current block identity;
conditional comes from the current block state's `conditional` property.

The typed world projection is `CommandBlockEntityDataState`: `command`,
`success_count`, optional rich `custom_name`, `track_output`, optional rich
`last_output`, `update_last_execution`, optional `last_execution`, `powered`,
`condition_met`, and `automatic`. Missing fields use the exact Vanilla defaults.
Only a block-entity type `minecraft:command_block` at a current command-block,
chain-command-block, or repeating-command-block state can open/refresh the screen.
Malformed typed fields reject the packet atomically through the existing world
apply-error path.

`AbstractCommandBlockEditScreen` uses an EditBox at `(width / 2 - 150, 50)` with
width `300`, suggestions anchored at y `72`, and seven visible rows. The bbb editor
reuses the existing `ChatEntryState` suggestion/result lifecycle and the
world-owned dispatcher; it parameterizes placement and row limit instead of
creating another grammar or server-response cache. Track-output controls whether
the previous output or `-` is shown. The mode cycle follows enum order. Done is
inactive until the matching authoritative update, sends the current six packet
fields once, clears local last-output display when tracking is off, and closes;
Cancel/Escape closes without sending.

The server handler checks gamemaster permission again, resolves the block entity,
changes the block identity/facing/conditional state, writes command and
track-output, clears output when disabled, applies automatic/mode-switch behavior,
and emits ordinary authoritative block/block-entity updates. The client does not
mutate canonical block state on submit. Existing system chat displays permission
or command-block-disabled diagnostics.

Authoritative sources are
`network/protocol/game/ServerboundSetCommandBlockPacket.java`,
`client/gui/screens/inventory/AbstractCommandBlockEditScreen.java`,
`client/gui/screens/inventory/CommandBlockEditScreen.java`,
`world/level/block/CommandBlock.java`,
`world/level/block/entity/CommandBlockEntity.java`,
`world/level/BaseCommandBlock.java`, `server/level/ServerPlayer.java`,
`client/multiplayer/ClientPacketListener.java`, and
`server/network/ServerGamePacketListenerImpl.java` in the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Add the typed protocol encoder and net transport, then add the typed world
   command-block projection and active editor lifecycle. Focused protocol/net/world
   tests cover exact bytes, all flag combinations, UTF-16 boundary rejection,
   Vanilla NBT defaults/rich output, matching refresh, wrong type/position, block
   invalidation, permission guard, and level reset.
2. Add the native editor and renderer HUD. Focused native/renderer tests cover
   disabled-to-enabled refresh, text selection and the UTF-16 limit, shared local
   and server suggestions at the editor anchor, all three cycles/toggles, rich
   previous output, Cancel/Escape, one-shot Done submission, invalidation close,
   and deterministic offscreen screen geometry.
3. Mark the packet row complete, remove only the closed unsupported TODO, append
   the milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `33894cb887` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native UI work; input/HUD
failures stop before coverage closeout. Failures are fixed forward without history
rewrites.
