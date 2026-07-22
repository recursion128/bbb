# Jigsaw block editor packet closure

## Goal and completion contract

Close all open coverage cells for
`play/serverbound/SERVERBOUND_SET_JIGSAW_BLOCK` and
`play/serverbound/SERVERBOUND_JIGSAW_GENERATE` with one native vertical path.
A permitted local player who uses a loaded jigsaw block opens its in-game
editor while the ordinary `UseItemOn` request still reaches the server. The
editor preloads the authoritative block-entity fields. Done sends the exact
SetJigsawBlock request once. Generate sends that same update first and then the
JigsawGenerate request once, in FIFO order. Cancel and Escape send nothing.
The server remains authoritative for permission, block-entity mutation,
template-pool lookup, structure generation, chunk mutation, and resulting
block/chunk updates.

Completion means both encoders match Vanilla 26.1 field order and types. The
world retains typed jigsaw NBT plus the active editor identity and derives the
joint control visibility from the canonical block orientation. The editor
enforces Vanilla resource-location validity, field limits, integer fallbacks,
the `0..=20` generation slider, keep-jigsaws toggle, joint rules, and exact
button guards. Focused semantic tests, the full gate, and the applicable
playable gate pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns typed joint/update/generate values, exact packed BlockPos,
  resource-location/string/VarInt/bool encoding, field bounds, and wire tests.
- `bbb-net` owns the two `NetCommand` transport arms and unique play-driver send
  paths. A single channel preserves Set-before-Generate order.
- `bbb-world` owns typed `minecraft:jigsaw` NBT data, block-orientation-derived
  editor state, permission/block/type guards, authoritative refresh,
  invalidation, and session reset.
- `bbb-native` owns field focus/caret editing, identifier validation, integer
  parsing, slider/toggle/joint interaction, ordered submission/cancellation,
  screen replacement, localization, and world-to-HUD projection.
- `bbb-renderer` owns immutable HUD values and deterministic widget, slider,
  tooltip, and text geometry.

This slice does not implement server-side template pools or jigsaw placement;
predict generated blocks; invent client success/failure feedback; render a
generation preview; add an in-game configuration surface; or close the whole
BlockEntityData row. It does not combine the structure, test, command-block,
or jigsaw editors into a generic form framework.

## Vanilla and integration contract

`ServerboundSetJigsawBlockPacket.STREAM_CODEC` writes packed `BlockPos`, name,
target, and pool as resource-location UTF strings, final state as UTF, joint as
the lowercase UTF value `rollable` or `aligned`, then selection priority and
placement priority as VarInts. `ServerboundJigsawGeneratePacket.STREAM_CODEC`
writes packed `BlockPos`, levels as VarInt, and keep-jigsaws as a bool. The Set
packet must be enqueued before Generate when the Generate button is activated.

The block entity NBT uses `name`, `target`, `pool`, `final_state`, `joint`,
`selection_priority`, and `placement_priority`. Missing or malformed resource
ids fall back to `minecraft:empty`; final state falls back to `minecraft:air`;
priorities fall back to zero. An absent/invalid joint follows
`StructureTemplate.getDefaultJointType`: ALIGNED when the block orientation's
front axis is horizontal and ROLLABLE when it is vertical. A malformed NBT
root rejects the update atomically; individual absent or wrongly typed fields
follow Vanilla `ValueInput`/Codec fallbacks. Ordinary BlockEntityData updates
are canonical and refresh the typed state without overwriting unsent native
edits in the same open screen.

`JigsawBlock.useWithoutItem` opens locally only for
`Player.canUseGameMasterBlocks`, while the ordinary use still reaches the
server. The active editor is bound to the loaded block position and closes on
block/type replacement, chunk unload, level reset, or screen replacement.
The `orientation` property controls only joint-widget visibility: `up_*` and
`down_*` fronts are vertical and show an active ROLLABLE/ALIGNED cycle;
horizontal fronts hide it while preserving the loaded joint value.

Pool, name, and target edit boxes are 300 pixels wide at y 20, 55, and 90 and
cap at 128 UTF-16 units. Final state is 300 pixels wide at y 125 and caps at
256 units. Selection and placement priority fields are 98 pixels wide at y
160 and cap at three UTF-16 units; invalid signed integer input maps to zero.
The joint control is 100 pixels wide at y 160. Labels render ten pixels above
their widgets. Done and Generate are enabled only while all three resource ids
parse through Vanilla identifier rules; final state is not part of that guard.

The level slider occupies `(center-154,185,100,20)`, begins at zero, and maps
mouse value with the Vanilla eight-pixel handle formula before flooring the
clamped interpolation to `0..=20`. Keep Jigsaws begins true at
`(center-50,185,100,20)`. Generate is at `(center+54,185,100,20)`, Done at
`(center-154,210,150,20)`, and Cancel at `(center+4,210,150,20)`. Done and Enter
send only Set and close. Generate sends Set, closes, then sends Generate.
Cancel/Escape close without mutating canonical state or sending either packet.

The server handler rechecks game-master permission and jigsaw block-entity
type. Set installs every request field, marks the entity changed, and emits a
block update. Generate looks up the authoritative pool and performs placement;
bbb never performs either effect locally.

Authoritative sources are
`network/protocol/game/ServerboundSetJigsawBlockPacket.java`,
`network/protocol/game/ServerboundJigsawGeneratePacket.java`,
`client/gui/screens/inventory/JigsawBlockEditScreen.java`,
`client/gui/components/AbstractSliderButton.java`,
`world/level/block/JigsawBlock.java`,
`world/level/block/entity/JigsawBlockEntity.java`,
`world/level/levelgen/structure/templatesystem/StructureTemplate.java`,
`client/player/LocalPlayer.java`, and
`server/network/ServerGamePacketListenerImpl.java` in the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Move the existing generate codec into a semantic jigsaw packet module, add
   typed Set encoding and both net transports, then add typed world NBT/editor
   state and lifecycle. Focused protocol/net/world tests cover exact bytes,
   both joint strings, resource and UTF-16 bounds, signed VarInt priorities,
   generation extremes, NBT defaults/wrong types/malformed rejection,
   orientation default and visibility, permission/type guards, authoritative
   refresh, block replacement, chunk unload, and level reset.
2. Add the native editor and renderer HUD. Focused native/renderer tests cover
   local open plus ordinary use, preload/editing, identifier validity, limits
   and integer defaults, joint visibility/cycle, exact slider mapping,
   keep-jigsaws, Done, Set-before-Generate ordering, invalid-button guards,
   Cancel/Escape zero-send, screen replacement, tooltips, and compact/wide
   geometry.
3. Mark both packet rows complete, remove only the closed unsupported entry,
   append the milestone index, archive this contract, review the integrated
   diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `6bcc1d6a1` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native UI work;
input/HUD failures stop before coverage closeout. Failures are fixed forward
without history rewrites.
