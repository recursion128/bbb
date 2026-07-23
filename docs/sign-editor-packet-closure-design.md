# Sign editor packet coverage closure

## Goal and completion contract

Close the remaining partial coverage cells for `ClientboundOpenSignEditorPacket`
and `ServerboundSignUpdatePacket`. An accepted open packet must snapshot the
canonical raw and filtered text of the selected sign side, and the native edit
screen must expose the same bounded text-editing, clipboard, IME, Done-button,
and invalid-lifecycle behavior as Vanilla before submitting exactly four lines.

Completion means the canonical sign state retains optional filtered messages,
both world rendering and edit preload select them from the startup text-filter
setting, normal and hanging signs enforce their respective measured line widths,
and Done, Escape, screen replacement, sign removal, and excess distance cannot
send more than one update for one editor lifetime. All packet coverage dimensions
are `complete`, the three finished sign TODOs are removed, and the full plus
playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns lenient `filtered_messages` decoding in sign block-entity
  NBT. The existing sign-update wire encoder remains authoritative.
- `bbb-world` owns raw and optional filtered sign-side state, the open-editor
  snapshot, sign-presence validation, and the Vanilla interaction-range check.
- `bbb-native` owns startup filter selection, measured text editing, selection
  and clipboard commands, IME preedit state, one-shot submission, and Done-button
  input.
- `bbb-renderer` owns the existing sign preview plus the Done button and IME
  preedit projection.

This slice does not implement server-side chat filtering, predict an accepted
sign update into canonical world state, add an in-game configuration screen, or
expand unrelated sign rendering fidelity such as bidirectional shaping, Unihex
fallbacks, glowing outlines, or decoration bars. It does not generalize NBT
decoding to other block-entity families.

## Vanilla and integration contract

`SignText.DIRECT_CODEC` decodes exactly four `messages` and a lenient optional
four-entry `filtered_messages`; a missing or malformed filtered list falls back
to the raw messages. Both `AbstractSignEditScreen` and in-world sign rendering
select `SignText.getMessage(index, Minecraft.isTextFilteringEnabled())`. The
client's command-line `client_text_filtering` value therefore becomes an input
publication value rather than a second world-state owner. The open-editor world
state snapshots both raw and effective filtered lines when the packet is applied,
so later block updates in the same network drain cannot change the screen's
initial buffer.

`AbstractSignEditScreen` keeps four strings and validates the whole candidate
with `font.width(candidate) <= sign.getMaxTextLineWidth()`. A normal
`SignBlockEntity` allows 90 pixels; `HangingSignBlockEntity` allows 60. Up selects
the previous line, Down and Enter select the next line, and every line change
moves the cursor to the end. Escape and the Done button close the screen.
`removed()` always emits exactly one `ServerboundSignUpdatePacket` containing the
four buffered lines and the selected side. The protocol encoder writes the
packed block position, side boolean, and four UTF strings without introducing a
screen-level character cap.

`TextFieldHelper` supplies Ctrl+A/C/V/X, Shift-preserving selection, word motion,
Home/End, and whole-candidate validation. Clipboard input removes carriage
returns and legacy formatting sequences matching
`(?i)\u00a7[0-9A-FK-OR]`; ordinary typed input accepts Vanilla chat characters.
IME commit text follows the same candidate validator. `IMEPreeditOverlay` keeps
the composing text separate from the committed line, uses the sign cursor as its
anchor, and applies the existing 5-pixel border, 4-pixel vertical gap, 2-pixel hot
area, and 300-millisecond caret blink conventions already represented by the
inventory preedit path.

The screen closes when the sign block entity disappears, the local player is
missing, or the player is no longer within block interaction range. Vanilla tests
the squared distance from the eye position to the block's unit AABB with the
strict comparison
`distance_squared < (block_interaction_range + 4.0) ^ 2`; the attribute fallback
is the Vanilla default 4.5. This validation runs during simulation pumping, not
only after keyboard input, and funnels through the same one-shot submit path.
Replacing the sign screen with an existing native dialog retains the repository's
established dialog-stack behavior while still submitting each removed sign-screen
lifetime once.

Authoritative local sources are
`net/minecraft/network/protocol/game/ServerboundSignUpdatePacket.java`,
`net/minecraft/client/gui/screens/inventory/AbstractSignEditScreen.java`,
`net/minecraft/client/gui/components/TextFieldHelper.java`,
`net/minecraft/client/gui/components/IMEPreeditOverlay.java`,
`net/minecraft/world/level/block/entity/SignBlockEntity.java`,
`net/minecraft/world/level/block/entity/HangingSignBlockEntity.java`,
`net/minecraft/world/level/block/entity/SignText.java`,
`net/minecraft/world/entity/player/Player.java`, and
`net/minecraft/ChatFormatting.java` in the local Vanilla 26.1 source tree.

## Serial stages and verification

1. Decode optional filtered sign messages, retain raw/filtered canonical world
   state, snapshot both variants on an accepted open packet, and add the
   sign-presence plus eye-to-AABB range query. Focused tests: sign NBT decoding,
   raw/filtered fallback and snapshot selection, removal, range boundary, and
   accepted/rejected open-editor application in `bbb-protocol` and `bbb-world`.
2. Publish the startup filter setting, replace the fixed character cap with
   measured whole-candidate validation, add Vanilla selection/clipboard/IME
   behavior, validate the editor every simulation pump, and render/activate Done
   plus preedit. Focused tests: `bbb-native` sign input, lifecycle, filter
   selection, mouse, clipboard, IME, runtime publication, and `bbb-renderer` sign
   projection regressions. This core cross-boundary milestone gets a full gate.
3. Mark every cell for both packets complete, remove the finished native-command,
   HUD, and block-entity TODOs, append the milestone index, review the integrated
   diff, and run `scripts/cargo-dev.sh gate` followed by
   `scripts/playable-gate.sh auto`.

The fully gated `465521cc9` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Protocol/world failures stop before native input changes;
input or renderer failures stop before coverage closeout. Failures are fixed
forward without rewriting history.
