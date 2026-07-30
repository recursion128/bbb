# Sound Caption Overlay Engineering Contract

Status: completed on 2026-07-29.

## Goal and completion

Close the active sound-caption gap with the observable Vanilla 26.1 subtitle
overlay behavior:

- observe successfully resolved local, positioned, entity-bound, jukebox, and
  newly started ambient-loop sounds at their actual playback or delayed-release
  edge;
- resolve the root `sounds.json` subtitle key through the current resource
  generation and merge equal subtitle Components while retaining recent sound
  locations;
- apply the listener-relative audible range, closest-location direction,
  notification-time expiry, and brightness decay rules; and
- render the bounded rows at Vanilla's right-bottom geometry with background,
  centered text, and conditional left/right arrows.

Completion requires semantic audio timing/range observations, native
merge/expiry/direction/localization tests, startup-option tests, renderer
layout and deterministic offscreen pixel tests, deletion of the active ledger
item, archival of this contract, a concise milestone index entry, and a passing
full gate at the final head.

## Authoritative behavior

- `SubtitleOverlay.extractRenderState` registers its sound listener only while
  `Options.showSubtitles` is enabled. The option defaults false.
- `SoundEngine.play` resolves the root event and selected sound before notifying
  listeners. Notification precedes category-volume rejection, buffer decode,
  and channel admission. Distance-delayed sounds notify only when the delayed
  play runs.
- `onPlaySound` ignores events without a subtitle Component. Equal Components
  share one row; refreshing removes an identical location and appends it with
  the current monotonic time. The first row's range remains its range.
- A linear, non-relative sound is audible only when the listener is strictly
  closer than `max(instanceVolume, 1) * Sound.attenuationDistance`; relative or
  non-attenuated sounds use infinite range. The closest recent location drives
  direction and brightness.
- Each location remains for `3000ms * notificationDisplayTime`. Brightness is
  `floor(clampedLerp(age/maxAge, 255, 75))`.
- The listener forward dot product must exceed `0.5` to count as in view.
  Otherwise positive listener-right dot draws `>`, negative draws `<`, and zero
  draws neither.
- Rows preserve first-seen subtitle order. The widest localized row determines
  the shared width, then adds the font widths of `"<"`, `" "`, `">"`, and one
  more space. With `halfWidth=width/2`, row `n` translates to
  `(guiWidth-halfWidth-2, guiHeight-35-n*10)`, fills
  `[-halfWidth-1,-5,halfWidth+1,5]` with the default 80%-opaque black text
  background, centers the text at `(-textWidth/2,-4)`, and places an arrow at
  the corresponding edge.
- `Gui.extractSubtitleOverlay` is distinct from title packet subtitles. With no
  screen it is deferred until after toast/debug extraction. With a screen it is
  submitted before that screen's own background/content. Hide-GUI suppresses it
  unless the current screen reports `isInGameUi`.

Authoritative local sources are
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/components/SubtitleOverlay.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/sounds/SoundEngine.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/Gui.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/screens/Screen.java`,
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/Options.java`, and
`AbstractSoundInstance.java`.

## Ownership and frozen interfaces

- `bbb-audio::KiraAudioRuntime` owns a bounded FIFO of
  `AudioPlaybackEvent { event_id, position, range }`. It records the event at
  the actual non-delayed or delayed-release play edge before mute/decode/channel
  rejection, records only newly started ambient loops, clears pending
  observations on resource-generation reset, and exposes one draining API.
- Existing `AudioCommand` and canonical `bbb-world` sound state remain
  unchanged. `NativeAudioRuntime` drains playback observations and maps the
  root event id to the current `SoundEventDefinition.subtitle`; sounds without
  a subtitle are discarded.
- A new main-thread `SoundSubtitleRuntime` owns non-serializable `Instant`
  timestamps, first-seen row order, recent locations, bounded overflow policy,
  expiry, listener geometry, localization, and projection to renderer rows.
  No wall-clock or subtitle state enters `bbb-world` or simulation publication.
- `bbb-renderer` gains bounded `HudSoundSubtitles` /
  `HudSoundSubtitleDirection` presentation types and one setter. It owns font
  measurement, shared row width, exact pixel geometry, draw sanitization, and
  whether the current renderer screen set requires before-screen or deferred
  ordering.
- Startup gains `--client-show-subtitles` (Vanilla default false) and
  `--notification-display-time` (`0.5..=10.0`, Vanilla default `1.0`). They are
  startup-only; no in-game configuration UI is added.
- The renderer determines screen ordering from its already projected screens.
  In-game UI means container/book/sign/command/structure/jigsaw/test editors;
  other projected screens are non-in-game. Subtitle visibility is
  `!hide_gui || in_game_ui_screen`.

## Bounds and fallback rules

- The audio playback FIFO, subtitle groups, and recent locations per group use
  explicit caps and drop oldest observations on overflow. Tests pin the caps
  and ordering. This is the intentional safety bound around Vanilla's growing
  lists.
- Non-finite positions, non-positive finite ranges, invalid display-time
  multipliers, empty/sanitized subtitle keys, or absent listener state do not
  produce a row. Infinite range remains valid.
- Missing subtitle keys fall back to the translation key, matching an
  unresolved translatable Component. Missing audio runtime or a disabled
  startup option projects no overlay.
- Resource reload changes subsequent root-event subtitle lookup and current
  localization. Existing groups retain their key and are re-localized against
  the current resource generation until they expire.

## Non-goals

- This slice does not implement title packet subtitles, directional/HRTF audio,
  the separate accessibility text-backdrop option, narrator announcements,
  sound-category sliders, or any in-game options screen.
- It does not change sound selection, gain, attenuation, streaming, cache,
  stop, loop, or entity-following semantics beyond exposing the listener
  observation at the already owned playback edge.
- It does not add a live playable fixture; deterministic semantic and GPU
  evidence is the acceptance surface.

## Serial stages and verification

1. **Audio observation and startup contract.** Add the bounded playback FIFO,
   exact local/spatial/range observations, delayed-release and ambient-start
   behavior, native subtitle-key lookup, and the two CLI options. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-audio playback_event`,
   `scripts/cargo-dev.sh test -p bbb-native audio_runtime`, and
   `scripts/cargo-dev.sh test -p bbb-native startup`.
2. **Native lifetime and direction projection.** Add the main-thread subtitle
   runtime, exact merge/refresh/expiry/brightness/audibility/direction rules,
   resource localization, and render-loop drain/projection wiring. Focused
   tests: `scripts/cargo-dev.sh test -p bbb-native sound_subtitle`.
3. **Renderer and closeout.** Add sanitized row presentation, exact ordering
   and layout tests, and deterministic offscreen pixels for background,
   brightness, arrows, and row stacking. Remove the ledger item, archive this
   contract, append the concise milestone index, run
   `scripts/cargo-dev.sh test -p bbb-renderer sound_subtitle`, then the standard
   per-commit checks and final full gate.

The fully gated `e618f3673` is the pre-start high-risk checkpoint. Each stage is
buildable and independently reviewable; failure is localized in FIFO timing,
subtitle-key resolution, native lifetime/vector math, renderer geometry, then
pixels. Failures are fixed forward before the next stage.
