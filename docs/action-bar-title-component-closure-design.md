# Action-bar and title Component closure

Status: active engineering contract.

## Goal and completion contract

Close the rich-Component gap for the native action-bar, title, and subtitle
surfaces. The three dedicated clientbound packets and the overlay-producing
`SystemChat` branch must preserve one exact trusted Component from protocol
decode through world ownership, then resolve its localized styled runs against
the selected resource generation and render them with the correct per-run font.

Completion requires:

- `ClientboundSetActionBarTextPacket`, `ClientboundSetTitleTextPacket`, and
  `ClientboundSetSubtitleTextPacket` decode their
  `ComponentSerialization.TRUSTED_STREAM_CODEC` field into
  `RichTextComponent`; no packet-specific String projection remains;
- `ActionBarState` and `HudTitleState` own those exact Components, including
  inherited style, siblings, translation payloads, click/hover payloads, and
  font identity, while keeping the existing Vanilla countdown and clear/reset
  behavior;
- `SystemChat(overlay=true)` delegates to the same world-owned action-bar
  setter, so the chat and dedicated-packet producers cannot diverge;
- native frame extraction localizes translation templates against the selected
  immutable resource generation and projects contiguous visual runs containing
  text, bold/italic/underline/strikethrough/obfuscated flags, color, and font;
- the renderer uses the shared selected-pack font atlas and a
  `HudFontGlyphBook` to measure, center, obfuscate, and draw each run with its
  requested font, using the default font only when the requested font is not
  available;
- action-bar position, 60-tick fade, optional jukebox color animation,
  title/subtitle 4x/2x scale, title fade windows, draw strata, shadow/effect
  ordering, and post-GUI-tick extraction timing remain unchanged;
- click and hover payloads remain exact in canonical state but do not create a
  hit target, matching Vanilla's non-interactive overlay/title surfaces;
- the applicable `canonical_apply` and `runtime_side_effect` cells for all
  three title-text packet rows, plus the `runtime_side_effect` cell for
  `CLIENTBOUND_SYSTEM_CHAT`, reach `complete` with semantic evidence; and
- the rich-title TODO and the overlay-system-chat rich-action-bar TODO are
  removed from `docs/unsupported-features.md`.

## Non-goals

- Bidirectional shaping and the unavailable `unihex` provider remain under
  their existing deferred restart conditions.
- The accessibility text-backdrop option remains a separate HUD todo; the
  current CLI/default value stays zero.
- This slice does not add interaction handling to action-bar/title text.
- Tab-list header/footer, server-list MOTD, lectern, boss-bar, chat tooltip, and
  other Component consumers keep their existing owners. This slice may expose
  a reusable run/font projection but does not migrate those surfaces.
- Jukebox now-playing production remains its existing separate HUD todo. The
  already-carried `animate_color` state must not regress.
- No in-game configuration surface is added.

## Boundaries and frozen integration API

Semantic owners:

- protocol: `crates/bbb-protocol/src/packets/client_state.rs`;
- canonical state: `crates/bbb-world/src/client/hud.rs`;
- localization/frame projection:
  `crates/bbb-native/src/component_text.rs` and
  `crates/bbb-native/src/runtime/render_extract.rs`;
- shared pure font/run values:
  `crates/bbb-render-types/src/hud_glyphs.rs`;
- selected-pack font upload and renderer ownership:
  `crates/bbb-native/src/hud_assets.rs`,
  `crates/bbb-renderer/src/renderer.rs`, and
  `crates/bbb-renderer/src/hud.rs`;
- coverage and gap ownership:
  `scripts/data/packet-coverage-overrides-26.1.json`,
  `docs/packet-coverage-26.1.json`,
  `crates/bbb-protocol/src/packet_coverage.rs`, and
  `docs/unsupported-features.md`.

Frozen types and calls:

- the three protocol packet structs expose `content: RichTextComponent`;
- `ActionBarState::content`, `HudTitleState::title`, and
  `HudTitleState::subtitle` use `RichTextComponent` directly rather than
  independently storing plain/styled/exact projections;
- `WorldStore` has one internal overlay-message setter accepting a
  `RichTextComponent`; both packet owners delegate to it;
- the renderer-facing rich overlay run is a dedicated pure value containing
  `HudStyledTextRun` visual data plus `Option<String>` font identity. Existing
  generic HUD run APIs are not widened merely to make this slice compile;
- `Renderer` retains a `HudFontGlyphBook` beside the existing default
  `HudFontGlyphMap`. `upload_hud_font_atlas` accepts either the full book or a
  default-only map for focused tests and rebuilds matching obfuscation pools at
  the resource transaction boundary;
- rich overlay/title measurement and geometry select glyph maps and
  obfuscation pools per run, but delegate glyph/style/effect emission to one
  internal geometry walk rather than copy it; and
- world-to-renderer data still crosses only the single `RendererFrame`
  submission.

Fallback behavior:

- an absent resource runtime resolves translatable text with the Component's
  exact fallback or key and carries the exact font id;
- an unknown/unavailable font id draws through the selected default font,
  matching the existing explicit font-book fallback;
- malformed trusted Components remain protocol decode errors under the shared
  Component limits and never mutate world state; and
- an absent title still suppresses subtitle drawing exactly as Vanilla does.

## Vanilla 26.1 evidence

- `ClientboundSetActionBarTextPacket.STREAM_CODEC`,
  `ClientboundSetTitleTextPacket.STREAM_CODEC`, and
  `ClientboundSetSubtitleTextPacket.STREAM_CODEC` use
  `ComponentSerialization.TRUSTED_STREAM_CODEC`.
- `ClientPacketListener.setActionBarText`, `setTitleText`, and
  `setSubtitleText` pass the packet Component directly to `Gui`.
- `ChatListener` overlay delivery calls `Gui.setOverlayMessage(message, false)`,
  the same owner as the dedicated action-bar packet.
- `Gui.setOverlayMessage`, `setTitle`, `setSubtitle`, `setTimes`,
  `clearTitles`, `tick`, `extractOverlayMessage`, and `extractTitle` define the
  canonical fields, countdowns, alpha formulas, position, scale, and ordering.
- `GuiGraphicsExtractor.textWithBackdrop` submits the Component itself.
- `Font` constructs its `StringSplitter` and glyph draws from
  `style.getFont()`, applies bold advance, obfuscation, per-style color, drop
  shadow, underline, and strikethrough.

## Serial stages and focused tests

The pre-start high-risk checkpoint is clean commit `7215e4b8d`; its full gate,
recorded playable fixture smoke, and P0 audit passed.

### Stage 1: protocol and world canonical ownership

- replace the three lossy packet String fields with exact
  `RichTextComponent`;
- migrate world action-bar/title state and the overlay system-chat producer;
- retain timing, clear/reset, serialization, and session-reset behavior; and
- add decode and world tests proving exact NBT/style/font/click/hover survival
  plus unchanged timing.

Focused commands:

- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-protocol title`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-world hud_text`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-world title`

Because this stage changes protocol decode shape, commit it clean and run a
full gate before advancing.

### Stage 2: localized projection and selected-font ownership

- add the dedicated renderer-facing Component run;
- make native projection resolve translations and all visual Style fields;
- retain the full selected-pack font book in the renderer transaction owner;
  and
- add focused resource-generation, unknown-font, and projection tests.

Focused commands:

- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-render-types font_book`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-native hud_action_bar_and_title`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-renderer hud_font`

Commit the stage clean. A full gate is required if focused tests expose an
integration failure; otherwise the next stage is the planned checkpoint.

### Stage 3: font-aware renderer and closeout

- route rich action-bar/title width and geometry through the per-run font book;
- preserve the existing alpha, positioning, scale, strata, effects, and shadow
  behavior with semantic geometry/pixel tests;
- update packet coverage, its reverse contract test, the feature-gap ledger,
  and the milestone archive; and
- regenerate/check the packet ledger.

Focused commands:

- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-renderer action_bar`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-renderer title`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-native hud_action_bar_and_title`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-protocol title_controls`
- `scripts/vanilla-source-tools.sh scripts/generate-packet-catalog.py --check`

After the clean closeout commit, run `scripts/cargo-dev.sh gate`,
`scripts/playable-gate.sh live`, `scripts/playable-gate.sh auto`, and
`scripts/audit-p0.sh`. A missing external Java/display/server prerequisite may
leave only the live portion not run; the fixture smoke, full gate, and P0 audit
must pass at the delivered HEAD.

## Failure localization and safe stopping

- Stage 1 failures are confined to protocol packet construction/decoding,
  world canonical state, or callers still expecting String.
- Stage 2 failures are confined to localization projection, shared pure run
  values, or resource-transaction font ownership.
- Stage 3 failures are confined to font-aware layout/geometry or coverage
  closeout.
- Every stage ends in a buildable reviewed commit. Failures are fixed forward;
  no commit or history rewrite is allowed.
- Safe stopping points are the clean stage commits after their required
  focused checks and checkpoint gate. No unrelated slice starts before this
  contract closes or reaches a genuine blocker.
