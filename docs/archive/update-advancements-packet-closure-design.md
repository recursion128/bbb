# Update-advancements packet coverage closure

## Goal and completion contract

Close the remaining decode, canonical-apply, and runtime-side-effect coverage
cells for `ClientboundUpdateAdvancementsPacket`. Trusted title and description
Components must survive decode and world storage without flattening, the
client advancement tree and progress map must retain the packet-defined display
and item-template data needed by the existing screen, and every Vanilla-
eligible completed progress update must enter a bounded native toast queue with
the exact display snapshot.

Completion means display title and description retain validated Component NBT,
plain text, and inherited styled runs; root tabs, widgets, hover boxes, and
toasts consume those runs; icon `ItemStackTemplate` values continue through the
shared component-aware item renderer rather than a packet-specific item schema;
reset/remove/add/progress ordering and unknown-parent/progress behavior remain
covered; non-reset completed progress entries obey both packet
`showAdvancements` and display `showToast`; advancement toasts use Vanilla's
five one-slot layout, 600 ms squared slide, 5000 ms fully-visible duration,
single-line versus multiline title transition, challenge completion sound, and
session/reconfiguration cleanup; all five packet coverage dimensions are
`complete`; the two finished advancement TODOs are removed; and the full plus
playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the exact update packet wire order, trusted rich
  Components, item-template component patch, frame ordinal, display flags,
  bounded collections/strings, identifier normalization, and trailing-byte
  rejection.
- `bbb-world` owns the serializable advancement tree, progress, exact display
  projections, counters, and session cleanup. Its completion snapshots enter
  the bounded, globally sequenced client-toast handoff shared with recipe
  notifications.
- `bbb-native` owns consumption of unseen toast events, the five-slot visible
  set and FIFO queue, wall-clock animation, rich screen/toast projection,
  component-aware icon projection, and ordered local UI sound commands.
- `bbb-renderer` owns the advancement-toast background, styled glyph and item
  submission, clipping, and right-edge/slot layout. The existing advancement
  window, tab, line, widget, hover, scroll, and scissor owners remain the only
  screen path.

This slice does not implement Mojang telemetry upload; the architecture
explicitly forbids collecting or uploading Mojang client telemetry. It does
not introduce a second item-component decoder, a new text layout engine,
notification display-time configuration UI, recipe/system/now-playing toast
generalization, bidi/unihex support, or unrelated advancement criterion
execution. The server remains authoritative for definitions and progress.

## Vanilla, wire, and integration contract

`ClientboundUpdateAdvancementsPacket` writes `reset`, an ordered
`AdvancementHolder` list, removed identifiers, progress entries, and
`showAdvancements`. Each advancement writes optional parent, optional
`DisplayInfo`, requirements, and `sendsTelemetryEvent`. `DisplayInfo` writes
trusted title and description Components, an `ItemStackTemplate`, frame enum,
an integer flag word (`1` background present, `2` show toast, `4` hidden),
optional background identifier, then x/y floats. Existing strict bounds,
identifier normalization, enum validation, and terminal-payload rejection stay
authoritative while both display Components switch to `RichTextComponent`.

`ClientAdvancements.update` clears on reset, removes before adding, normalizes
and stores progress only for known nodes, and handles each progress entry after
the tree mutations. When the packet is not a reset and normalized progress is
done, Vanilla records its external telemetry callback independently of toast
eligibility. bbb intentionally omits that external callback. A toast is queued
only when packet `showAdvancements` is true and the node has display data whose
`showToast` is true. This check is performed for every completed progress entry,
not only a false-to-true edge. The event copies the display and icon so later
tree removal cannot mutate an already queued toast.

Canonical client-toast state assigns each toast event a monotonically
increasing sequence across notification families and retains a fixed recent
window large enough for one maximum packet drain. Native remembers the last
consumed sequence per client-level generation, ingests each unseen event once
in sequence order, and records an explicit drop counter if publication lag ever
overtakes the retained window. Reset packets do not enqueue, while
Login/Respawn/reconfiguration/disconnect clear both the canonical recent
handoff and native visible/queued presentation.

`ToastManager` exposes five vertical slots. `AdvancementToast` occupies one
slot for 5000 ms multiplied by the notification-time option; bbb has no in-game
option surface and therefore uses Vanilla's default multiplier `1.0`. Each
instance slides for 600 ms with squared progress and reverses from its current
visible portion when hiding. A challenge toast requests
`minecraft:ui.toast.challenge_complete` when admitted to a visible slot;
visibility transitions use the existing `minecraft:ui.toast.in` and
`minecraft:ui.toast.out` local UI sounds, with at most one visibility-change
sound total per update pass as in `ToastManager`.

The toast is 160 by 32 using `minecraft:toast/advancement`. A title that fits
one 125-pixel line renders the localized frame label at `(30, 7)` and title at
`(30, 18)`. Multiline titles show the frame label at `(30, 11)` through 1500 ms,
cross-fade over 300 ms, then center the wrapped title lines vertically. The
component-aware icon renders at `(8, 8)`. Existing repo-native styled-run and
item-model paths are reused for screen and toast presentation.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundUpdateAdvancementsPacket.java`,
`net/minecraft/advancements/Advancement.java`, `DisplayInfo.java`,
`AdvancementTree.java`,
`net/minecraft/client/multiplayer/ClientAdvancements.java`,
`net/minecraft/client/gui/screens/advancements/AdvancementsScreen.java`,
`AdvancementTab.java`, `AdvancementWidget.java`,
`net/minecraft/client/gui/components/toasts/ToastManager.java`, and
`AdvancementToast.java` in the local Vanilla 26.1 source tree.

## Serial stages and verification

1. Decode exact rich display Components and carry them through ordered world
   tree/root/widget summaries plus deterministic toast events. Focused tests:
   protocol rich NBT/style/inheritance, invalid/trailing input, display flags
   and item-template components; world reset/remove/add/progress ordering,
   exact rich projections, repeated-done eligibility, reset/no-show/no-display
   suppression, unknown progress, event bounds, and session reset. Because this
   changes protocol decode, the stage gets a full gate.
2. Add the native five-slot queue and animation, advancement-toast renderer
   surface/background/icon/rich text, challenge and transition sounds, and rich
   advancement screen labels. Focused tests: once-only ingestion and lag drop,
   FIFO slot admission, five-slot saturation, zero/half/full/reversed slide,
   5000 ms visibility, single/multiline fade, exact styled runs, challenge sound,
   component-aware icon, hide-GUI/session cleanup, renderer geometry and
   sanitization. This cross-runtime presentation milestone gets a full gate.
3. Mark the packet row complete, remove the two finished advancement TODOs,
   append the milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `c5bf2b926` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Rich-decode failures stop before world migration; world event
or reset failures stop before presentation; queue, renderer, or audio failures
stop before coverage closeout. Failures are fixed forward without rewriting
history.
