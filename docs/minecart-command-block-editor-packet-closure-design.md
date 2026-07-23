# Minecart command block editor packet closure

## Goal and completion contract

Close all five coverage cells for
`play/serverbound/SERVERBOUND_SET_COMMAND_MINECART` with one native vertical
path. A permitted local player who interacts with a command-block minecart
opens its in-game editor while the ordinary entity interaction still reaches
the server. The editor preloads the synced command and previous output, keeps
the client entity's track-output choice, and offers command suggestions. Done
and Enter send the exact request once; Cancel and Escape send nothing. The
server remains authoritative for permission, entity type, command-block
enablement, command mutation, output execution, and metadata acknowledgements.

Completion means the encoder matches Vanilla 26.1 field order and limits. The
world projects the two generated metadata accessors into a typed command-block
minecart view, owns the per-entity client track-output bit, and binds the active
editor to one concrete entity incarnation. The native editor preserves unsent
command edits across matching metadata refreshes, updates previous output from
the server, closes on entity removal/replacement and screen replacement, and
renders the exact minecart command editor controls. Focused semantic tests, the
full gate, and the applicable playable gate pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the typed request, VarInt/string/bool encoding, editor
  command bound, and exact wire tests in the command-block packet module.
- `bbb-net` owns the `NetCommand` transport arm and unique play-driver send
  path.
- `bbb-world` owns generated-accessor metadata projection, per-incarnation
  track-output state, editor identity/revision, permission/type guards,
  authoritative refresh, entity lifecycle invalidation, and level reset.
- `bbb-native` owns local interaction open, command focus/caret/suggestions,
  track-output interaction, submission/cancellation, screen replacement,
  localization, and world-to-HUD projection.
- `bbb-renderer` owns immutable HUD values and deterministic widget/text
  geometry.

This slice does not execute commands, predict a successful server mutation,
invent disabled-command feedback, add command-block configuration UI, or
close unrelated Interact/SetEntityData/SystemChat packet cells. It does not
combine block and minecart command editors into a new generic form framework;
it reuses their existing command-entry and rendering primitives where their
Vanilla behavior is identical.

## Vanilla and integration contract

`ServerboundSetCommandMinecartPacket.STREAM_CODEC` writes the entity id as a
VarInt, the command with default `FriendlyByteBuf.writeUtf`, and track-output
as a bool. `AbstractCommandBlockEditScreen` limits the command edit box to
32,500 UTF-16 units, below the wire's 32,767-character default. No block
position, command-block mode, conditional bit, automatic bit, sequence, or
interaction hand belongs in this packet.

`MinecartCommandBlock.DATA_ID_COMMAND_NAME` is the generated STRING accessor
with default `""`; `DATA_ID_LAST_OUTPUT` is the generated COMPONENT accessor
with `CommonComponents.EMPTY`. Matching SetEntityData updates are canonical.
They refresh the world projection and previous-output display without
overwriting an unsent native command in the same open editor. `trackOutput` is
not synced metadata: `BaseCommandBlock` initializes it to true and the client
screen mutates that concrete minecart's local command-block object. bbb keeps
the corresponding bit in a narrow per-entity component, resets it on entity
incarnation replacement, and clears the local previous output when the user
turns tracking off and submits.

`MinecartCommandBlock.interact` returns PASS without game-master permission.
With permission, the client opens the editor and the normal Interact request
still reaches the server. The active editor records the protocol id and entity
incarnation and closes when that entity is removed, replaced, changes type, or
the level/session is cleared. Opening any other native screen closes it;
opening it closes the block command, structure, jigsaw, test-instance, sign,
container, book, advancements, stats, debug, pause, and dialog surfaces by the
existing single-screen rules.

The command edit box is `(center-150,50,300,20)`. The previous-output field is
`(center-150,150,276,20)` and its track-output cycle is
`(center+130,150,20,20)`. The translated title is centered at y 20; the command
label starts at `(center-149,40)` and the previous-output label at
`(center-149,140)`. The track button displays `O` when enabled and `X` when
disabled. Minecart editors have no mode, conditional, or automatic buttons.
Done and Cancel retain the shared positions
`(center-154,height/4+132,150,20)` and
`(center+4,height/4+132,150,20)`.

Done or Enter sends entity id, the edited command, and the current local
track-output bit, then closes. If tracking is off, it clears only the local
last-output projection, matching `AbstractCommandBlockEditScreen.onDone`.
Cancel and Escape close without sending and without changing the command; a
track-output toggle remains local to the entity exactly as Vanilla mutates the
underlying `BaseCommandBlock` before cancellation. Command suggestions use the
existing commands-only request, seven-row list, keyboard, mouse, scroll, and
narration behavior.

`ServerGamePacketListenerImpl.handleSetCommandMinecart` rechecks game-master
permission and resolves the target only when it is still a
`MinecartCommandBlock`. It sets command and track-output, clears last output
when tracking is disabled, and calls `onUpdated` only when command blocks are
enabled. Only that server update changes the synced command/last-output
metadata; bbb does not apply the submitted command locally or manufacture an
acknowledgement.

Authoritative sources are
`network/protocol/game/ServerboundSetCommandMinecartPacket.java`,
`client/gui/screens/inventory/MinecartCommandBlockEditScreen.java`,
`client/gui/screens/inventory/AbstractCommandBlockEditScreen.java`,
`world/entity/vehicle/minecart/MinecartCommandBlock.java`,
`world/level/BaseCommandBlock.java`, `client/player/LocalPlayer.java`, and
`server/network/ServerGamePacketListenerImpl.java` in the local Vanilla 26.1
source tree.

## Serial stages and verification

1. Add the typed protocol encoder and net transport, then add the typed world
   metadata/editor projection and lifecycle. Focused protocol/net/world tests
   cover representative and multi-byte entity ids, exact wire order, UTF-16
   boundary, generated metadata ids/types/defaults, permission/type guards,
   track-output persistence, matching metadata refresh, same-id replacement,
   removal, and level reset.
2. Add the native editor and renderer HUD. Focused native/renderer tests cover
   local open plus ordinary Interact, preload and unsent-edit preservation,
   commands-only suggestions, the 32,500-unit bound, track-output display and
   local lifecycle, Done/Enter one-shot submission, Cancel/Escape zero-send,
   screen replacement, and deterministic compact/wide geometry.
3. Mark the packet row complete, remove only the closed unsupported entry,
   append the milestone index, archive this contract, review the integrated
   diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `6840743e8` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native UI work;
input/HUD failures stop before coverage closeout. Failures are fixed forward
without history rewrites.
