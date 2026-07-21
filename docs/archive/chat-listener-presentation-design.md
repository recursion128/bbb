# ChatListener Presentation Engineering Contract

This document freezes the ownership, Vanilla 26.1 evidence, serial phases, and
validation for the native `ChatListener` presentation slice. Signed transport,
remote-session validation, `DeleteChat`, synchronized `ChatType`, and exact rich
Component retention are already established prerequisites. This slice turns
those values into the one ordered, visible client chat path; it does not add a
second chat authority or an in-game options screen.

Baseline before this slice is full/live-gated commit `141e3d810`. At that tip,
`WorldStore` validates signed player messages and retains rich player/disguised
Components, but immediately appends every packet to `client_chat.messages`.
It does not apply Vanilla receiveability, secure-only, social/filter, delayed
delivery, trust tags, reporting-log, HUD, interaction, or narration semantics.
`SystemChat` also retains only its plain summary.

## Semantic Goal and Completion Criteria

One simulation-owned client-chat state must implement Vanilla 26.1's complete
message acceptance order and publish an immutable presentation generation. The
render owner consumes that generation to draw the same ordered, rich chat lines
and tags, while narration and platform actions travel through the existing
lossless effect sequence before their matching publication becomes visible.

Completion requires all of the following:

- Player, disguised, remote system, client system, and overlay messages retain
  their exact validated Component plus inherited styled/click/hover projection.
- Player trust distinguishes secure, modified, and not-secure messages using
  signature presence, the seven-minute client expiry boundary, signed-content
  containment, and custom-font unsigned-content modification. Secure-only mode
  suppresses only Vanilla's `NOT_SECURE` class.
- The actual delivery attempt, not packet receipt, applies player/system
  receiveability, hidden/blocked sender, fully/partially filtered content, and
  `hideMatchedNames`. A processed signature enters last-seen acknowledgement
  with the exact `wasShown` result only after that attempt.
- A nonzero message delay uses a bounded FIFO and monotonic milliseconds.
  Immediate delivery, one-message timed delivery, zero-delay flush, explicit
  next-message acceptance, pause compensation, configuration flush, and
  signature deletion match `ChatListener` ordering. No queue growth is
  unbounded.
- Visible chat retains at most 100 entries and the reporting log retains the
  newest 1,024 player/remote-system events with stable ids. Client-system and
  overlay messages do not enter the reporting log. Deletion still removes a
  pending delayed signature first or replaces the visible entry no earlier
  than 60 GUI ticks.
- The HUD wraps rich runs at the configured chat width, shows the newest ten
  lines, uses Vanilla age/focus alpha and background ordering, preserves
  source/trust tag identity, and exposes the delayed-queue indicator. Opening
  chat shows retained lines without changing canonical message order.
- Hover and click hit testing uses the same wrapped glyph runs as rendering.
  Supported server-safe click events reuse the native dialog/platform action
  boundary; disallowed `open_file` remains inert. A resource or level
  generation change cannot make a stale hit target actionable.
- Player/disguised chat narration uses the resolved narration decoration and
  chat narrator gate; validation errors use queued system-chat narration;
  remote/client system and overlay messages use their corresponding Vanilla
  system methods. A blocked render owner cannot lose or duplicate narration.
- Renderer and control keep using the exact same immutable world Arc and
  generation. Effect-before-publication, no-op publication suppression,
  blocked-render isolation, shutdown/join, and old-generation immutability do
  not regress.

## Non-goals

- Do not add abuse-report composition/submission screens, a social-interactions
  screen, Mojang account sign-in, or a second account/block-list service. The
  bounded reporting log and hidden/blocked-id query surface are completed here;
  obtaining an authenticated Services block list remains the existing external
  account/social boundary.
- Do not add in-game configuration UI. Chat visibility, secure-only, delay,
  link policy, and name-filter options are startup configuration.
- Do not change signed-chat wire formats, key refresh, remote validation,
  command signing, tick cadence, or server authority.
- Do not make the renderer, narrator process, control API, or winit event loop a
  mutable chat owner. Do not poll live simulation state from the render thread.
- Do not implement missing general font providers or every Component style as a
  hidden prerequisite. Unsupported fonts/styles retain exact Component data and
  explicit fallback; they are not silently flattened out of canonical state.
- Do not treat log output, counters, or a plain-text control snapshot as proof
  of visible rich HUD behavior.

## Ownership and Integration Contract

- `bbb-protocol` owns exact wire Components and validated value projections.
  `SystemChat` joins the existing player/disguised rich projection. Style value
  types may retain hover/font identity needed by presentation, but contain no
  runtime decisions.
- `bbb-world` owns the sole mutable `ClientChatState`: acceptance configuration,
  signature/chain state, bounded delayed FIFO, 100 visible entries, deletion
  timers, 1,024-entry reporting ring, hidden/blocked ids, delivery sequence, and
  language-neutral narration/action payloads. It accepts explicit monotonic and
  epoch time from the caller; it never reads a global clock or backend.
- Packet receipt creates one validated pending delivery value. Immediate and
  delayed delivery call the same acceptance function. That function alone may
  add a visible message, report-log event, last-seen entry, and narration
  request. Entity/profile removal cannot retarget a queued sender UUID.
- `bbb-native` initializes startup policy, advances the listener on raw client
  ticks, queues returned acknowledgements, and puts narration/platform actions
  into the existing sequence-numbered lossless effect batch. It owns chat-screen
  focus/scroll/hover runtime, but not message contents.
- `bbb-renderer` owns only wrapped-line scratch, font measurements, draw
  geometry, tag sprites, and current hit-test layout derived from one immutable
  publication. Cache keys include world level/session identity, message
  delivery revision, font/resource generation, width/scale, and focus/scroll.
- `bbb-control` may expose the canonical queue/log/tag counters from the paired
  publication. It gets no independent chat copy and no mutation path outside
  the simulation mailbox.

Every bounded collection rejects or evicts before allocation growth: visible
messages 100, reporting events 1,024, delayed deliveries 1,024, wrapped HUD
lines 100, and retained styled runs/component values remain subject to the
protocol Component limits. Delayed overflow processes the oldest delivery
instead of dropping a signed message or silently skipping acknowledgement.

## Authoritative Vanilla 26.1 Evidence

- `ClientPacketListener.handlePlayerChat`, `handleDisguisedChat`,
  `handleSystemChat`, `handleDeleteChat`, `markMessageAsProcessed`, and
  `handleConfigurationStart` define validation, listener calls,
  acknowledgement, deletion, and queue-flush ordering.
- `ChatListener.tick`, `setMessageDelay`, `handleMessage`,
  `showMessageToPlayer`, `handleChatMessageError`, `handleSystemMessage`, and
  `handleOverlay` define the delayed FIFO, captured versus live options,
  receiveability, filtering, blocking, logging, and narration branches.
- `ChatTrustLevel.evaluate` and `PlayerChatMessage.hasExpiredClient` define
  secure/modified/not-secure classification and the seven-minute client expiry.
- `FilterMask.applyWithFormatting` defines partial-mask `#` runs and the dark
  gray `chat.filtered` hover style.
- `ChatAbilities.selectVisibleMessages` and `Minecraft.computeChatAbilities`
  map FULL/SYSTEM/HIDDEN to client/player/server source visibility while always
  retaining client-system messages.
- `ChatComponent.addMessage`, `splitLines`, `deleteMessage`,
  `processMessageDeletionQueue`, and `extractRenderState` define the 100-entry
  queues, wrapping, fade/background/tag order, focus behavior, and 60-tick
  deletion floor.
- `GuiMessage`, `GuiMessageTag`, `LoggedChatMessage`, `ChatLog`, and
  `ReportingContext` define source/tag fields and the 1,024-event reporting ring.

## Serial Phases and Validation

1. **Canonical listener owner.** Preserve rich `SystemChat`; add startup policy,
   trust/filter/receiveability decisions, a bounded delayed-delivery owner,
   acceptance-time acknowledgements, reporting ring, narration requests, reset
   behavior, diagnostics, and old-publication isolation tests. Focused protocol,
   world, net/native dispatcher, startup, publication, and lifecycle tests run
   before a normal commit. Because packet application and acknowledgement
   ordering change, run a milestone full gate.
2. **Rich HUD and tags.** Add renderer-facing chat values, stable localization
   and rich decoration, exact bounded wrapping/fade/background/tag layout,
   delayed-queue indicator, render scratch reuse, and before/after projection
   tests. Verify unchanged publications reuse the exact world Arc and that HUD
   extraction cannot mutate it. Run renderer/native focused tests and a normal
   commit.
3. **Interaction and narration effects.** Retain hover/font identity needed by
   chat, make renderer layout and simulation input share one hit-test contract,
   reuse the existing safe Component action boundary, and deliver localized
   chat/system narration through the lossless effect FIFO. Test stale targets,
   blocked render, skipped redraws, narrator gates, URL prompt policy, clipboard,
   suggest/run command, and no duplicate delivery. Run a milestone full gate.
4. **Coverage and closeout.** Update packet/registry/entity evidence,
   `unsupported-features`, architecture, and `goal-archive`; retain account
   block-list/report-screen boundaries precisely. Run generator checks, affected
   full crate tests, formatting, diff/docs checks, P0 audit, and complete diff
   review before the final normal commit.
5. **Delivery gates.** Run the final workspace full gate at delivered `HEAD`,
   build `/tmp/bbb-target-main/debug/bbb-native` from that exact commit, verify
   the binary timestamp, then run explicit `scripts/playable-gate.sh live` with
   the fixed Vanilla 26.1/Weston environment and the policy-required `auto` gate.

## Failure Localization and Safe Stops

Phase 1 changes canonical ordering without rendering; its focused tests can
distinguish trust, queue, filter, acknowledgement, reporting, reset, and retained
generation failures. Phase 2 consumes only immutable values and localizes
layout failures to extraction/renderer. Phase 3 adds platform effects only after
the acceptance and visual contracts are gated. Coverage changes occur only
after behavior is complete.

The safe stops are clean, full-gated tips after phases 1 and 3. Once a phase
starts, stop only after its focused checks and normal commit, an explicit real
blocker, or a normal fix-forward commit restoring a buildable state. Never
deliver a state where both receipt-time and delivery-time acknowledgement paths
are selectable, or where renderer/narrator state can mutate canonical chat.
