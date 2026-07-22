# Test-instance block editor packet closure

## Goal and completion contract

Close all five coverage cells for
`play/serverbound/SERVERBOUND_TEST_INSTANCE_BLOCK_ACTION` and the remaining
canonical/runtime cells for
`play/clientbound/CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS` with one native
vertical path. A permitted local player who uses a loaded test-instance block
opens its in-game editor while the ordinary `UseItemOn` request still reaches
the server. Opening sends exactly one `INIT`; valid or invalid test-id edits
send `QUERY`; only a still-active editor consumes the paired status/optional
size response. Reset, Save, Run, and Done send their exact action once; Cancel
and Escape send nothing. The server remains authoritative for registry lookup,
test execution, structure placement/save/export, block-entity mutation, and
feedback.

Completion means the request encoder matches Vanilla 26.1 exactly, including
the full `TestInstanceBlockEntity.Data.STREAM_CODEC` shape emitted by the
production editor. The world retains typed nested block-entity data and an
active editor identity without treating the uncorrelated status response as
global rule state. The editor preserves rich status/error Components, validates
the 128-character resource id, clamps each submitted size axis to `1..=48`,
cycles all rotations, negates include-entities into the wire
`ignore_entities`, enforces Vanilla Save eligibility, and renders deterministic
native geometry. Focused semantic tests, the full gate, and the applicable
playable gate pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns typed action/rotation/request data, exact packed BlockPos,
  VarInt/boolean/optional fields, fixed outgoing CLEARED/no-error editor data,
  rich clientbound status decode, bounds, and wire tests in a semantic
  test-instance packet module.
- `bbb-net` owns the `NetCommand` transport arm and unique play-driver send path.
- `bbb-world` owns typed `minecraft:test_instance_block` NBT data, the active
  editor position/revision, permission/block/type guards, screen-scoped rich
  status and optional-size application, invalidation, and session reset.
- `bbb-native` owns text/numeric focus, caret editing, INIT/QUERY sequencing,
  action buttons, submission/cancellation, screen replacement, localization,
  and world-to-HUD projection.
- `bbb-renderer` owns the immutable HUD value and deterministic widget/text
  geometry.

This slice does not implement server-side GameTest execution, structure
storage/export, a development-only Export button, test-instance beacon/box
world rendering, a test catalog configuration UI, or close the whole
clientbound BlockEntityData row. `SharedConstants.IS_RUNNING_IN_IDE` is false
for the production Vanilla client, so the native production surface exposes
Reset/Save and Run/Done/Cancel but no Export control.

## Vanilla and integration contract

`ServerboundTestInstanceBlockActionPacket.STREAM_CODEC` writes packed
`BlockPos`, a VarInt action id, then `TestInstanceBlockEntity.Data`: optional
test resource key, three signed VarInts for size, VarInt rotation id, boolean
`ignoreEntities`, VarInt status id, and optional Component. Action ids are
`INIT=0`, `QUERY=1`, `SET=2`, `RESET=3`, `SAVE=4`, `EXPORT=5`, `RUN=6`.
Rotation ids are NONE, CLOCKWISE_90, CLOCKWISE_180, COUNTERCLOCKWISE_90 in that
order. The production screen constructor always sends status CLEARED and no
error Component; bbb's native action surface keeps those fields fixed instead
of exposing a second mutable authority.

`TestInstanceBlock.useWithoutItem` opens locally only for
`Player.canUseGameMasterBlocks` and a test-instance block entity, while the
ordinary use succeeds and reaches the server. The typed NBT projection reads
the nested `data` compound: optional `test`, required three-int `size`,
`rotation`, `ignore_entities`, `status`, and optional rich `error_message`.
Malformed present data rejects the block-entity update atomically; absent data
uses the block entity constructor defaults (no test, zero size, NONE, false,
CLEARED, no error).

The screen is 316 pixels wide around `width / 2 - 158`: id input at y 40,
status/info region at y 70 with eight nine-pixel rows, five equal size/rotation/
entity widgets at y 160, Reset/Save at y 185, and Run/Done/Cancel at y 210.
The id field has a 128 UTF-16-unit cap; each size field has a 15-unit input cap,
parses a decimal i32, maps invalid input to 1, and clamps to `1..=48`. Save is
active only for a syntactically valid identifier with rotation NONE. Include
Entities starts as `!ignoreEntities` and is negated on send. Reset, Save, and
Run send then close. Done sends SET then closes. Cancel/Escape close without a
packet.

Initialization sends INIT after fields are populated. Every id edit sends QUERY
with either the parsed key or an empty optional key; an invalid id immediately
shows the local red invalid-id Component. The server replies with
`ClientboundTestInstanceBlockStatus(Component, Optional<Vec3i>)`. Because that
packet carries no position or request id, `ClientPacketListener` applies it only
when `TestInstanceBlockEditScreen` is current. The active editor prepends its
typed block-entity error Component using the Vanilla failed-description shape,
preserves the response Component tree, and replaces the three size fields only
when the response includes a size.

The server handler rechecks game-master permission and block-entity type.
INIT/QUERY only resolve the synchronized test-instance registry and return
status/optional template size. Other actions first install the supplied data;
RESET/SAVE/EXPORT/RUN perform server-only effects, SET only updates data, and
ordinary block updates carry authority back to the client.

Authoritative sources are
`network/protocol/game/ServerboundTestInstanceBlockActionPacket.java`,
`network/protocol/game/ClientboundTestInstanceBlockStatus.java`,
`client/gui/screens/inventory/TestInstanceBlockEditScreen.java`,
`world/level/block/TestInstanceBlock.java`,
`world/level/block/entity/TestInstanceBlockEntity.java`,
`client/player/LocalPlayer.java`,
`client/multiplayer/ClientPacketListener.java`, and
`server/network/ServerGamePacketListenerImpl.java` in the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Add the typed protocol encoder/rich status decoder and net transport, then
   add typed world block-entity/editor state and screen-scoped response apply.
   Focused protocol/net/world tests cover exact action bytes, all rotations,
   optional key, signed VarInts, UTF-16/resource validation, NBT defaults and
   malformed rejection, permission/type guards, response scoping, optional
   size, invalidation, chunk unload, and level reset.
2. Add the native editor and renderer HUD. Focused native/renderer tests cover
   local open plus ordinary use, single INIT, QUERY ordering for valid/invalid
   ids, rich error/status composition, response size replacement, numeric parse
   and clamp, rotation/include toggles, Save gating, one-shot action close,
   Cancel/Escape zero-send, screen replacement, and deterministic compact/wide
   geometry.
3. Mark both packet rows complete, remove only the closed unsupported/deferred
   entries, append the milestone index, archive this contract, review the
   integrated diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `3ded68044` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native UI work; input/HUD
failures stop before coverage closeout. Failures are fixed forward without
history rewrites.
