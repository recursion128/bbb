# Code of Conduct Screen Presentation Engineering Contract

Status: active engineering contract.

## Goal and completion

Replace the custom uploaded 500x300 code-of-conduct bitmap with a repo-native
projection of Vanilla 26.1 `CodeOfConductScreen` / `WarningScreen`:

- render the ordinary screen background, centered title, exact wrapped server
  text area, optional scrollbar, "do not notify again" checkbox, and
  Acknowledge / Disconnect buttons with native pack sprites and fonts;
- preserve hover, focus, keyboard activation, mouse wheel, scrollbar drag, and
  resize-dependent layout;
- map Acknowledge to the existing accept request with the current checkbox
  value and Disconnect to the existing decline request; and
- keep the screen modal, non-Escape-closeable, and connected to the existing
  server-list acceptance persistence boundary.

Completion requires semantic layout/input tests at compact, ordinary, and
scrolling surface sizes; renderer command/scissor tests; deterministic
offscreen pixels for normal, hovered/focused, checked, and scrolled states;
removal of the active ledger item and old bitmap owner; archival of this
contract; a concise milestone index entry; and a passing full gate at the final
head.

## Authoritative behavior

- `CodeOfConductScreen` subclasses `WarningScreen`, supplies a bold
  `multiplayer.codeOfConduct.title`, literal packet text,
  `multiplayer.codeOfConduct.check`, and a title-newline-content narration.
- `WarningScreen.init` builds one centered vertical layout with spacing 8:
  title, a `FittingMultiLineTextWidget` whose cell has 12 pixels of padding,
  then a vertical footer with spacing 8. The text widget starts at
  `(width - 100, height - 100)` and minimizes to content height when no
  scrollbar is needed.
- The footer contains a checkbox and a horizontal button row with spacing 8.
  Both default buttons are 150x20. The checkbox box is 17x17, has 4 pixels of
  label spacing, and uses selected/highlighted sprite combinations.
- The text area uses 4-pixel inner padding, a one-pixel inset scissor,
  nine-pixel line/scroll increments, and the standard text-field background.
  Its six-pixel scrollbar uses the standard scroller/background sprites; thumb
  height is `clamp(height^2 / contentHeight, 32, height - 8)`.
- Acknowledge calls the result consumer with `true` and persists the exact
  current checkbox selection; Disconnect calls it with `false` and clears the
  stored acceptance. Escape never closes the screen.

Authoritative local sources are:

- `net/minecraft/client/gui/screens/multiplayer/CodeOfConductScreen.java`
- `net/minecraft/client/gui/screens/multiplayer/WarningScreen.java`
- `net/minecraft/client/gui/components/FittingMultiLineTextWidget.java`
- `net/minecraft/client/gui/components/AbstractTextAreaWidget.java`
- `net/minecraft/client/gui/components/AbstractScrollArea.java`
- `net/minecraft/client/gui/components/Checkbox.java`
- `net/minecraft/client/gui/layouts/FrameLayout.java`
- `net/minecraft/client/gui/layouts/LinearLayout.java`

## Ownership and frozen interfaces

- `bbb-world` remains the sole canonical owner of the packet text and text
  hash. No cursor, scroll, focus, checkbox, or renderer state moves into world.
- `bbb-native::code_of_conduct_overlay` remains the screen-local input owner.
  Replace its fixed rectangles and bitmap rasterizer with state for checkbox
  selection, scroll amount, scrollbar drag, focus, and pointer hover. Its pure
  layout function consumes surface size plus the renderer font measurer.
- The existing `CodeOfConductControlRequest::Accept { remember }` and
  `Decline` surface remains unchanged. Acknowledge reads `remember` from the
  checkbox; the checkbox itself sends no request.
- `bbb-renderer` gains a typed `HudCodeOfConductScreen` and a semantic
  `hud/code_of_conduct.rs` owner. The screen carries pre-wrapped styled lines,
  exact widget rectangles, scroll state, hover/focus/selected flags, and text
  needed for native rendering. The renderer sanitizes bounded text/geometry
  and draws through existing font, button, text-field, checkbox, scroller,
  scissor, and menu-background owners.
- Remove `Renderer::set_hud_code_of_conduct_overlay`, the uploaded overlay
  texture field, `CodeOfConductOverlayRenderer`, and the hand-authored bitmap
  font. Render extraction sets the typed screen every publication generation;
  no resource upload occurs when only text/hash or interaction state changes.

## Fallback and bounds

1. Missing selected font glyphs use the existing font book fallback; packet
   text is never uppercased, ASCII-truncated, or replaced by a private glyph
   table.
2. Missing optional widget sprites omit only that sprite layer while retaining
   text, hit geometry, and control requests.
3. Screen dimensions and text/line counts are sanitized to existing HUD
   bounds. Scroll amount is finite and clamped to the derived maximum after
   resize or content change.
4. A newly received code hash resets checkbox, scroll, drag, and focus state.
   Acceptance/dismissal of one hash never hides a later hash.
5. Surfaces too small for the nominal layout retain bounded rectangles and a
   scrollable text area; they do not allocate an unbounded bitmap or panic.

## Non-goals

- This slice does not change packet decoding, the Java string hash, connection
  gating, server-list serialization, or the accept/decline net commands.
- It does not add in-game configuration, generalize every warning screen, or
  alter unrelated dialog/death/reporting screens.
- Narrator scheduling may reuse the existing modal-screen platform boundary;
  this slice does not redesign the global narration state machine.

## Serial stages and verification

1. **Typed layout and input state.** Add the pure dynamic layout, checkbox,
   focus, scroll/wheel/drag, resize reset/clamp, and exact accept/decline
   projection. Focused test:
   `scripts/cargo-dev.sh test -p bbb-native code_of_conduct`.
2. **Native renderer surface.** Add the typed renderer screen, sanitization,
   native widget/font/scissor draws, resource-owner reuse, command ordering,
   and offscreen pixels; remove the uploaded bitmap path. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-renderer code_of_conduct` and the native
   command from stage 1.
3. **Closeout.** Remove the ledger item, archive this contract, append the
   milestone index, run focused checks, docs lint, formatting, diff-check and
   complete diff review, then commit and run the full gate.

The final full gate is the engineering checkpoint. A playable gate is not
required because packet authority and connection commands do not change; the
existing control-request tests plus deterministic native input and GPU pixels
are the acceptance surface.

## Failure localization and safe stops

Layout/input failures compare surface-derived rectangles, wrapping, maximum
scroll, hover/focus, checkbox state, and emitted request before renderer code.
Renderer failures compare sanitized screen state, command/scissor order,
sprite availability, text geometry, then pixels. The clean contract commit and
the buildable stage-1 commit are safe stops. Any failure is fixed forward;
history is not rewritten.
