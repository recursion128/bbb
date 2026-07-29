# Bidirectional Text Shaping Engineering Contract

## Goal and completion

Close the active `Bidirectional shaping` font gap with the same ordering boundary
used by vanilla 26.1:

- select the paragraph fallback direction from the active pack language metadata;
- apply ICU4J `ArabicShaping(8)` semantics before Unicode bidirectional ordering;
- retain per-code-unit style, font, and interaction ownership while producing visual
  glyph order, including RTL mirroring;
- wrap and edit text in logical order, then reorder each completed display line;
- use the visual glyph sequence for rendered width, alignment, hit regions, and
  offscreen pixels across HUD and world-text consumers; and
- keep a resource reload atomic across the language-direction owner, font owners,
  and derived wrapped-text caches.

Completion requires focused unit and offscreen tests for LTR, RTL, mixed text,
Arabic joining and Lam-Alef compaction, style boundaries, supplementary codepoints,
mirroring, wrapping, and interaction hit regions. The active ledger item is removed
only after all production text entry points are audited, the engineering contract is
archived, and the required full and live gates pass.

## Authoritative behavior

- `net/minecraft/client/resources/language/FormattedBidiReorder.java` creates a
  `SubStringSource`, shapes with `new ArabicShaping(8)`, resolves ICU `Bidi` with
  default level `126` or `127`, and emits visual runs.
- `net/minecraft/network/chat/SubStringSource.java` stores one style per UTF-16
  code unit. It shapes the plain string without reshaping the style vector, splits
  each bidi run on those style indexes, and for odd runs reverses both style-run
  order and codepoints while applying `UCharacter.getMirror`.
- `net/minecraft/client/StringSplitter.java` measures and splits logical
  `FormattedText`; callers convert each completed line through
  `Language.getVisualOrder`.
- `net/minecraft/client/gui/Font.java` uses the selected language direction for
  plain-string drawing and consumes already reordered `FormattedCharSequence` for
  components.
- ICU4J 77.1 `ArabicShaping.java` is the exact shaping reference. The implementation
  is restricted to option `8`: logical input, Lam-Alef resize, letter shaping, no
  digit conversion.

Local vanilla 26.1 sources are authoritative for the call order and ownership.
The Unicode bidirectional algorithm and mirroring data are supplied by versioned
Rust dependencies compatible with the workspace Rust 1.80 floor. Arabic shaping is
an owned narrow implementation validated against the local ICU4J 77.1 jar; generic
Arabic reshapers are not an acceptable substitute because their extended-letter and
transparent-mark behavior differs.

## Ownership and frozen interfaces

- `bbb-pack::LanguageCatalog` owns `bidirectional: bool`.
  `PackRoots::load_client_language_catalog(code)` sets it from the resolved pack
  metadata entry for `code`; plain `load_language_catalog` and byte-only test
  constructors retain an LTR default.
- `bbb-render-types::text_visual_order` owns the pure UTF-16 shaping and visual-order
  algorithm. Its public operation accepts logical `(char, metadata)` values where
  metadata is cloneable and comparable, and returns visual values without knowing
  renderer styles. Metadata follows vanilla's UTF-16 indexing through Lam-Alef
  compaction. Empty or malformed input never panics and degrades to the original
  logical values.
- `bbb-renderer::Renderer` owns the selected default direction and exposes one
  direction update operation. Updating it rebuilds direction-derived chat data.
  The field is part of the renderer resource-reload checkpoint and rollback.
- HUD wrapping and editable buffers stay logical. Common formatted-text geometry
  converts a completed line to visual order before measuring and emitting glyphs.
  Chat click/hover ownership is carried as metadata through that conversion.
- Sign, display-entity, map, name-tag, debug, and other world-text render paths use
  the same pure operation after their own logical line split/truncation. No second
  bidi or Arabic-shaping implementation is allowed.
- `bbb-native` propagates the prepared `LanguageCatalog` direction during initial
  asset setup and every resource transaction. A failed transaction restores the
  exact prior direction and derived caches.

## Non-goals

- `unihex` / CJK font sources remain deferred under their existing reopen criterion.
- OpenType layout, kerning, ligatures other than ICU option-8 Lam-Alef, and font
  fallback redesign are outside this slice.
- Editable value storage, selection indexes, word movement, clipboard text, and
  protocol payloads remain logical. This slice does not add an in-game language or
  configuration UI.
- New protocol or canonical world state is not introduced.

## Serial stages and verification

1. **Direction and pure visual-order core.** Add the pack owner, versioned Unicode
   dependencies, exact option-8 Arabic shaper, bidi/mirroring transform, and
   ICU-derived fixed vectors. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-pack language` and
   `scripts/cargo-dev.sh test -p bbb-render-types text_visual_order`.
   This Cargo/core-architecture checkpoint ends with a clean commit and full gate.
2. **HUD and reload integration.** Add renderer direction state, transaction
   capture/rollback, common formatted geometry and visual-width helpers, and chat
   interaction mapping. Audit all HUD text geometry callers and add focused renderer
   and native resource-reload tests plus deterministic offscreen assertions. End
   with a clean commit and full gate.
3. **World-text integration.** Apply the common line transform to sign, display,
   map, name-tag, debug, and remaining non-HUD consumers without changing logical
   storage/wrap rules. Run focused renderer/item-model/native tests, commit clean,
   and run a full gate.
4. **Closeout.** Run production-entry scans, remove the active ledger item, archive
   this contract, add the milestone index entry, and extend deterministic live
   evidence when the playable fixture can exercise the selected RTL pack. Run
   documentation checks, focused tests, full gate, and the applicable playable gate.

Every stage also runs `cargo fmt --all --check`, `git diff --check`, full diff
review, and `scripts/docs-lint.sh` when documentation changes.

## Failure localization and safe stops

Pure shaping failures stop at stage 1 and compare hexadecimal UTF-16 output against
the local ICU4J jar. Bidi failures compare resolved visual runs independently from
Arabic shaping. Renderer failures compare pure visual values, generated quads,
interaction ranges, and finally offscreen pixels in that order. Reload failures
compare the direction owner and derived caches before inspecting GPU resources.

Safe stops are the clean, fully gated commits at the end of stages 1–3. No later
stage begins after a failed gate, and fixes are committed forward without rewriting
history.
