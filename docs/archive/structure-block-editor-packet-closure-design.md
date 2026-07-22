# Structure block editor packet closure

## Goal and completion contract

Close all five coverage cells for
`play/serverbound/SERVERBOUND_SET_STRUCTURE_BLOCK` with one native vertical
path. A permitted local player who uses a loaded structure block opens its
in-game editor while the ordinary `UseItemOn` request still reaches the
server. The editor preloads the authoritative block-entity fields and exposes
the Vanilla mode-specific controls. Done, Save, Load, and Detect each send
their exact update type once; Cancel and Escape send nothing. The server
remains authoritative for permission, structure storage, scanning and world
mutation, and ordinary block-entity updates plus system chat carry results
back to the client.

Completion means the request encoder matches Vanilla 26.1 field order,
ordinals, signed-byte coordinates, float bits, VarLong seed, and flag packing.
The world retains typed structure-block NBT state and an active editor identity
without predicting server operations. The editor enforces the 128-UTF-16-unit
name/data limits and Vanilla resource-name character filter, normalizes
offsets to `-48..=48`, sizes to `0..=48`, integrity to `0.0..=1.0`, and invalid
numeric input to Vanilla defaults before submission. It provides all four
modes, the Alt-enabled DATA cycle, all mirrors and rotations, entity/strict/
air/bounding-box toggles, exact mode visibility, and deterministic native
geometry. Focused semantic tests, the full gate, and the applicable playable
gate pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns typed update/mode/mirror/rotation/request values, exact
  packed BlockPos and payload encoding, bounds, flag packing, and wire tests in
  a semantic structure-block packet module.
- `bbb-net` owns the `NetCommand` transport arm and unique play-driver send
  path.
- `bbb-world` owns typed `minecraft:structure_block` NBT data, the active
  editor position/revision, permission/block/type guards, authoritative
  refresh, invalidation, and session reset.
- `bbb-native` owns focus and caret editing, numeric parsing/clamping, mode and
  option interaction, submission/cancellation, screen replacement,
  localization, and world-to-HUD projection.
- `bbb-renderer` owns the immutable HUD values and deterministic widget/text
  geometry.

This slice does not implement server-side structure storage, scanning or
placement; predict success/failure; render the structure bounding box or
invisible-air markers in the 3D world; add an in-game configuration surface;
or close the whole clientbound BlockEntityData/SystemChat rows. It does not
combine the jigsaw, test-block, or minecart-command-block editors into a
generic form abstraction.

## Vanilla and integration contract

`ServerboundSetStructureBlockPacket.STREAM_CODEC` writes packed `BlockPos`,
VarInt enum ordinals for update type and mode, a UTF string, three signed
offset bytes, three signed size bytes, VarInt mirror and rotation ordinals, a
UTF string, big-endian float integrity, VarLong seed, then one byte with
ignore-entities bit `1`, show-air bit `2`, show-bounding-box bit `4`, and
strict bit `8`. Update ordinals are UPDATE_DATA, SAVE_AREA, LOAD_AREA,
SCAN_AREA. Mode ordinals are SAVE, LOAD, CORNER, DATA; mirrors are NONE,
LEFT_RIGHT, FRONT_BACK; rotations are NONE, CLOCKWISE_90, CLOCKWISE_180,
COUNTERCLOCKWISE_90.

The block entity NBT uses `name`, `author`, `metadata`, `posX/Y/Z`,
`sizeX/Y/Z`, legacy enum strings for `rotation`, `mirror`, and `mode`, booleans
`ignoreEntities`, `strict`, `powered`, `showair`, and `showboundingbox`, float
`integrity`, and long `seed`. Missing values use the Vanilla constructor/load
defaults: empty strings, offset `(0,1,0)`, zero size, NONE mirror/rotation,
DATA mode, ignore entities true, strict/powered/show-air false,
show-bounding-box true, integrity 1, and seed 0. A malformed NBT root rejects
the update atomically; individual absent or wrongly typed fields follow
Vanilla `ValueInput.get*Or` / Codec `orElse` fallback behavior. The block
state's `mode` property and the typed NBT mode identify the same
structure-block family; server updates remain the canonical refresh.

`StructureBlockEntity.usedBy` opens locally only for
`Player.canUseGameMasterBlocks`, while the ordinary use still reaches the
server. The active editor is bound to the loaded block position and closes on
block/type replacement, chunk unload, level reset, or screen replacement. A
matching BlockEntityData packet refreshes canonical state; it does not
overwrite unsent native edits within the same open screen.

The name edit is 300 pixels wide at y 40. Offset fields occupy three 80-pixel
slots at y 80; size, integrity/seed, or metadata reuse y 120 by mode. Mode,
mirror/rotation, and action controls occupy y 185; Done and Cancel occupy y
210. SAVE shows name/offset/size, Save, Detect, Include Entities, and Show Air.
LOAD shows name/offset/integrity/seed, Load, Include Entities, Strict, Mirror,
four rotations, and Show Bounding Box. CORNER shows only name. DATA shows only
metadata. Holding Alt while cycling mode includes DATA; the ordinary cycle is
SAVE, LOAD, CORNER. Include Entities is the negation of wire
`ignore_entities`.

The name filter permits lowercase ASCII letters, digits, underscore, dash,
and dot everywhere, one colon only before any slash, and slash only after the
colon position. The name and metadata fields cap at 128 UTF-16 units; numeric
fields cap at 15 units and seed at 31. Invalid integer and seed text map to
zero; invalid integrity maps to one. Production input clamps offset/size and
integrity to the same ranges enforced by the server decoder before encoding,
so the typed request cannot rely on host-language narrowing. Done sends
UPDATE_DATA and closes. Save is effective only in SAVE, Load only in LOAD, and
Detect only in SAVE; each sends then closes. Cancel/Escape restores no
canonical state because native edits are screen-local and sends no packet.

The server handler rechecks game-master permission and structure-block entity
type, installs every request field, validates the parsed structure name, then
performs only the requested SAVE/LOAD/SCAN effect. It marks the block entity
changed, emits a block update, and reports translated success/failure through
system chat. bbb never performs these effects locally.

Authoritative sources are
`network/protocol/game/ServerboundSetStructureBlockPacket.java`,
`client/gui/screens/inventory/StructureBlockEditScreen.java`,
`world/level/block/StructureBlock.java`,
`world/level/block/entity/StructureBlockEntity.java`,
`world/level/block/state/properties/StructureMode.java`, `world/level/block/Mirror.java`,
`world/level/block/Rotation.java`, `client/player/LocalPlayer.java`, and
`server/network/ServerGamePacketListenerImpl.java` in the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Add the typed protocol encoder and net transport, then add typed world
   block-entity/editor state and lifecycle. Focused protocol/net/world tests
   cover exact representative bytes, every enum ordinal and flag, UTF-16
   bounds, coordinate/integrity bounds, VarLong extremes, NBT defaults and
   malformed rejection, permission/type/mode guards, matching authoritative
   refresh, block replacement, chunk unload, and level reset.
2. Add the native editor and renderer HUD. Focused native/renderer tests cover
   local open plus ordinary use, field preload and editing, name filtering,
   numeric defaults/clamps, normal/Alt mode cycles, exact per-mode visibility,
   mirror/rotation/options, guarded one-shot actions, Cancel/Escape zero-send,
   screen replacement, and deterministic compact/wide geometry.
3. Mark the packet row complete, remove only the closed unsupported entry,
   append the milestone index, archive this contract, review the integrated
   diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `ae73f6201` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native UI work;
input/HUD failures stop before coverage closeout. Failures are fixed forward
without history rewrites.
