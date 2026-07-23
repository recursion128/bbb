# Test block editor packet closure

## Goal and completion contract

Close all five coverage cells for
`play/serverbound/SERVERBOUND_SET_TEST_BLOCK` with one native vertical path. A
game-master player who uses a test block opens the in-game editor from the
canonical block-entity mode and message while the ordinary block use still
reaches the server. The screen exposes all four modes, hides but retains the
message in START mode, enforces the Vanilla edit-box bound, and sends the exact
request only from Done. Cancel and Escape send nothing.

Completion means the encoder matches Vanilla 26.1 field order and ids; the
world owns typed test-block block-entity data and an active editor snapshot;
native input owns message editing, focus, mode cycling, submission,
cancellation, and single-screen replacement; and the renderer reproduces the
deterministic widget and label geometry. The server remains authoritative for
permission, current block-entity type, mode/message mutation, block-state
mutation, and acknowledgements. Focused semantic tests, the full gate, and the
applicable playable gate pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns `TestBlockMode`, the typed request, packed BlockPos / mode
  VarInt / string encoding, the 128-UTF-16-unit editor bound, and exact wire
  tests.
- `bbb-net` owns the `NetCommand` transport arm and unique play-driver send
  path.
- `bbb-world` owns typed NBT projection, permission/type guards, the copied
  editor snapshot, authoritative world refresh, and level reset.
- `bbb-native` owns local use-to-open, message text editing, mode cycle, Done,
  Cancel/Escape, screen replacement, localization, and world-to-HUD
  projection.
- `bbb-renderer` owns immutable HUD values and deterministic widget/text
  geometry.

This slice does not execute tests, simulate redstone or `triggered`, predict a
successful server mutation, invent disabled-command feedback, add a startup or
in-game configuration surface, close unrelated UseItemOn/BlockUpdate/
BlockEntityData packet cells, or combine the existing editor implementations
into a generic form framework.

## Vanilla and integration contract

`ServerboundSetTestBlockPacket.STREAM_CODEC` writes `BlockPos.STREAM_CODEC`,
`TestBlockMode.STREAM_CODEC`, then `ByteBufCodecs.STRING_UTF8`. The position is
the standard packed 64-bit block position. The mode codec writes its stable id
as a VarInt: START=0, LOG=1, FAIL=2, ACCEPT=3. The packet string permits the
default 32,767-character wire bound, but `TestBlockEditScreen` sets the edit
box maximum to 128 Java `String.length()` units, so the repo-native request
rejects more than 128 UTF-16 units before encoding. No action enum, flags,
sequence, hand, or block-state id belongs in this packet.

`TestBlockEntity` persists root `mode`, `message`, and `powered` fields. Mode is
the lowercase codec name (`start`, `log`, `fail`, `accept`); a missing or
invalid mode falls back to FAIL during load, message defaults to `""`, and
powered defaults to false. The typed world projection retains all three
canonical fields even though this editor only changes mode and message. The
test block's `mode` block-state property drives its visible model and is
updated by the server-side block entity when mode changes.

`TestBlock.useWithoutItem` returns PASS for a missing/wrong block entity or a
player without game-master permission. With permission it opens the editor on
the client, returns SUCCESS, and the normal UseItemOn request still reaches the
server. `TestBlockEditScreen` copies position, mode, and message at open rather
than retaining the block entity. Accordingly, unsent native edits are not
overwritten by a matching BlockEntityData refresh; level/session reset and
normal single-screen replacement close the editor. Done closes immediately,
so later BlockUpdate and BlockEntityData packets are the only acknowledgement
and populate the next open; bbb does not mutate the submitted mode/message
locally.

The message edit box is `(center-152,80,240,20)`, with its translated label at
`(center-153,70)`. Both are visible for LOG, FAIL, and ACCEPT and hidden for
START without clearing the message. The title `block.minecraft.test_block` is
centered at y 10. The current mode's detailed message is drawn at
`(center-153,174)`. The mode cycle is `(center-154,185,50,20)` and follows enum
order START → LOG → FAIL → ACCEPT → START. Done is
`(center-154,210,150,20)` and Cancel is `(center+4,210,150,20)`. The English
fallbacks are `Test Block`, `Message:`, `Start`/`Log`/`Fail`/`Accept`, and the
four `test_block.mode_info.*` strings from the local Vanilla language file.
The screen is non-pausing in-game UI.

Only the Done button calls `onDone`; Vanilla has no TestBlockEditScreen Enter
submission override. Done sends position, current mode, and the retained
message once, then closes. Cancel, Escape, and ordinary close send nothing.
START mode still sends the retained hidden message. Message selection,
caret movement, and insertion use the existing native single-line text-edit
primitives.

`ServerGamePacketListenerImpl.handleSetTestBlock` rechecks game-master
permission, resolves the current position only when it still contains a
`TestBlockEntity`, calls `setMode`, `setMessage`, and `setChanged`, then sends a
block update from the old state to the block entity's new state. The client
applies those server packets through its existing canonical block and
block-entity paths; an invalid or stale request has no local acknowledgement.

Authoritative sources are `network/protocol/game/GameProtocols.java`,
`network/protocol/game/ServerboundSetTestBlockPacket.java`,
`client/gui/screens/inventory/TestBlockEditScreen.java`,
`world/level/block/state/properties/TestBlockMode.java`,
`world/level/block/TestBlock.java`, `world/level/block/entity/TestBlockEntity.java`,
`client/player/LocalPlayer.java`, and
`server/network/ServerGamePacketListenerImpl.java` in the local Vanilla 26.1
source tree, plus `assets/minecraft/lang/en_us.json` for fallback text.

## Serial stages and verification

1. Add the typed protocol encoder and net transport, then typed world
   block-entity/editor state. Focused protocol/net/world tests cover all four
   mode ids, exact wire order, packed signed position, UTF-16 boundary,
   NBT defaults and malformed fallback, permission/type guards, authoritative
   world refresh without editor-copy replacement, block-state acknowledgement,
   and level reset.
2. Add the native input owner and renderer HUD. Focused native/renderer tests
   cover local open plus ordinary UseItemOn, preload, unsent-edit preservation,
   START visibility with retained message, the 128-unit bound, full mode cycle,
   Done one-shot submission without local prediction, Cancel/Escape/Enter
   zero-send, screen replacement, and deterministic compact/wide geometry.
3. Mark the packet row complete, remove only the closed unsupported entry,
   append the milestone index, archive this contract, review the integrated
   diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `254b43646` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native UI work;
input/HUD failures stop before coverage closeout. Failures are fixed forward
without rewriting history.
