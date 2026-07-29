# Enchantment Table Presentation Engineering Contract

Status: completed 2026-07-29.

## Goal and completion

Close the active `EnchantmentMenu` presentation gap with the exact observable
Vanilla 26.1 screen behavior:

- tick and interpolate the GUI book's open/page-flip animation while the
  enchanting screen is active;
- render that book through an isolated GUI picture-in-picture color/depth
  target at Vanilla's rectangle, scale, transform, texture, light, and layer;
- derive the three seeded enchanting-language labels from the synchronized
  enchantment seed and the resource-pack `minecraft:alt` font;
- preserve enabled, disabled, and hovered option sprites plus their exact text
  colors; and
- render the hovered clue, level/lapis costs, and insufficient-resource colors
  from synchronized menu properties, registry definitions, player experience,
  and lapis state.

Completion requires deterministic animation and name-generation tests,
structural projection tests for every option state and tooltip branch,
renderer command-order tests, an offscreen pixel readback for the GUI book and
overlaid option strata, removal of the active ledger item, archival of this
contract, a concise milestone index entry, and a passing full gate at the final
head.

## Authoritative behavior

- `EnchantmentMenu` registers ten ordered data slots: costs `0..=2`,
  enchantment seed `3`, clue holder ids `4..=6`, and clue levels `7..=9`.
  `clickMenuButton` requires the option cost, `index + 1` lapis and player
  levels unless the player has infinite materials.
- `EnchantmentScreen.tickBook` compares the input stack, advances `flipT` with
  two `nextInt(4)` draws until it lies outside `flip ± 1`, stores previous
  open/flip values, approaches open by `0.2`, clamps it to `[0, 1]`, and applies
  the damped page acceleration. `extractBook` interpolates previous/current
  values by the frame partial tick.
- `GuiBookModelRenderer` renders the shared `BookModel` and
  `entity/enchantment/enchanting_table_book` texture into a private color/depth
  PIP target. Its pose is `translate(centerX, 17, 0) · scale(40,40,-40) ·
  Ry(180) · Rx(25) · translate((1-open)·(0.2,0.1,0.25)) ·
  Ry(-(1-open)·90-90) · Rx(180)`, under `Lighting.Entry.ENTITY_IN_UI` and
  full-bright light.
- `EnchantmentNames` resets one `LegacyRandomSource` to the synchronized seed
  each frame. Each nonzero option consumes `nextInt(2) + 3` words followed by
  one bounded draw per word from the fixed Vanilla word catalog, then truncates
  the `minecraft:alt` run to `86 - width(costText)` pixels.
- `EnchantmentScreen.extractBackground` draws the base texture, then the book,
  then each option. Unavailable options use disabled sprites and half-bright
  enchanting text; available hovered options use the highlighted sprite and
  yellow text. `extractRenderState` adds the registry-backed enchantment clue
  and the exact experience/lapis/level tooltip lines only for a valid clue.

Local sources under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/inventory/EnchantmentScreen.java`,
`EnchantmentNames.java`,
`net/minecraft/client/gui/render/pip/GuiBookModelRenderer.java`,
`net/minecraft/client/model/object/book/BookModel.java`, and
`net/minecraft/world/inventory/EnchantmentMenu.java` are authoritative.

## Ownership and frozen interfaces

- `bbb-world` remains the canonical owner of the open container, its ten
  synchronized data values, registry holder order/definitions, inventory
  stacks, game mode, and player experience. No animation cursor or GPU state is
  added to `WorldStore`.
- `bbb-native::input::enchantment_screen` owns one screen-local animation state
  keyed by the active enchantment container id and exact input stack. It
  advances once per raw client tick in the existing screen-tick phase. Vanilla
  seeds this purely local random source from process entropy; bbb uses a fixed
  deterministic `LegacyRandomSource` seed while preserving every bounded draw
  and transition.
- The same native module owns the pure seeded word selection and option
  presentation projection. It consumes menu values and a read-only
  `HudFontGlyphBook`; resource localization stays in `NativeItemRuntime`.
- `bbb-renderer::HudInventoryTextLabel` gains an optional whole-label font id.
  `None` retains the current default-font behavior; enchanting labels use
  `minecraft:alt`. Measurement and draw both resolve through the same immutable
  font book and shared atlas.
- `bbb-renderer::HudEntityPreview` gains an optional inventory background-layer
  insertion point. Existing entity previews retain their current
  after-all-background behavior; the enchanting book is inserted immediately
  after the base texture and before option sprites.
- The native projection reuses `EntityModelKind::EnchantingBook`, the existing
  book texture atlas entry, full-bright instance light, and the existing
  isolated entity-preview PIP target. It cancels the world-only book root
  translation/tilt and folds the exact GUI rigid transform into the preview
  translation/quaternion; no second book mesh, texture owner, or render
  pipeline is introduced.
- Tooltip clue identity resolves holder ids against the current synchronized
  enchantment registry and its typed tooltip definitions. Missing/invalid clue
  holders emit no option tooltip rather than inventing a Vanilla identity.

## Fallback rules

1. A missing `NativeItemRuntime` uses the existing English localization
   fallbacks and current-session registry resource identity where available.
2. A missing `minecraft:alt` font map falls back to the renderer's default font
   exactly like other unavailable Component font ids; seeded word selection
   remains stable.
3. A missing book texture/atlas owner records the existing entity submission
   fallback and leaves the static screen/options visible.
4. Missing clue definition data may use the synchronized holder resource id's
   Vanilla translation key; an unknown custom holder without a definition
   emits no fabricated clue text.
5. Absent experience or lapis state disables the option for presentation; it
   never makes an unaffordable option appear enabled.

## Non-goals

- This slice does not change packet decoding, container button commands,
  enchanting result authority, slot prediction, enchantment selection, or
  registry synchronization.
- It does not change the in-world enchanting-table block-entity animation or
  create a second book model/texture/pipeline.
- It does not generalize every GUI PIP producer, complete loom banner
  presentation, or close unrelated container/HUD gaps.
- It does not add in-game configuration UI or alter resource-pack font
  admission policy.

## Serial stages and verification

1. **Deterministic screen state.** Add the native screen-local book state,
   exact client-tick synchronization, seeded word catalog/projection, and pure
   option/tooltip inputs. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native enchanting_screen` and
   `scripts/cargo-dev.sh test -p bbb-world enchantment`.
   Keep this stage buildable; if consumer-free projection APIs trigger the
   workspace deny-warnings policy, integrate them directly with stage 2 rather
   than suppressing the warnings.
2. **HUD/PIP integration and closeout.** Add custom-font inventory labels,
   interleaved book PIP submission, exact option layers/tooltips, structural
   ordering tests, and deterministic offscreen readback. Remove the active
   ledger item, archive this contract, and append the concise milestone index.
   Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native enchant`,
   `scripts/cargo-dev.sh test -p bbb-renderer enchant`, and the exact offscreen
   readback test. Run formatting, docs lint, diff-check, and full diff review,
   commit clean, then run the full gate.

The final full gate is the engineering checkpoint. A playable gate is required
only if the implementation changes input command routing or the existing live
inventory scenario; structural hover/click tests and deterministic GPU
readback are the presentation acceptance surface otherwise.

## Failure localization and safe stops

Animation failures compare the per-tick `open/oOpen/flip/oFlip/flipT/flipA`
transition before any renderer state. Name failures compare random draw order,
selected words, alt-font widths, then truncation. Tooltip failures compare the
ten menu data slots and holder resolution before localization or colors.
Renderer failures compare preview rectangle/insertion metadata, decomposed GUI
transform, resident book texture, then PIP pixels and final option strata.

The integrated implementation commit is the safe stop. Any gate failure is
fixed forward before another slice; history is not rewritten.
