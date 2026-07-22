# Boss-event packet coverage closure

## Goal and completion contract

Close the remaining decode, canonical-apply, and runtime-side-effect coverage
cells for `ClientboundBossEventPacket`. A trusted Component name must survive
decode and world storage without flattening, active bars must retain Vanilla's
packet-add order, progress updates must render through the exact 100 ms
wall-clock interpolation, and all three aggregate properties must reach their
observable lightmap, fog/sky, and End boss-music consumers.

Completion means add/update names retain exact validated Component NBT and
inherited styled runs, replacing an existing id preserves its insertion slot,
remove then add appends it, repeated progress packets restart interpolation
from the value visible at that instant, presentation time never enters the
serializable world, boss fog hides the sky in addition to clamping atmospheric
fog and End-flash light, End boss music replaces another active music track and
restarts after natural completion while requested, all five packet coverage
dimensions are `complete`, the two finished boss-bar TODOs are removed, and
the full plus playable gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the six operation wire shapes, trusted rich Component
  validation, enum ordinals, property bits, and trailing-byte rejection.
- `bbb-world` owns one serializable insertion-ordered collection of active
  bars, exact name projections, target progress and deterministic instance /
  progress revisions, styles, properties, counters, and aggregate queries.
- `bbb-native` owns non-serializable 100 ms presentation interpolation, rich
  HUD projection, sky suppression, and the client-tick boss-music selector.
- `bbb-renderer` continues to own boss-bar layout, sprite selection, styled
  glyph drawing, lightmap input, fog uniforms, and sky submission.
- `bbb-audio` continues to own backend playback identity and exposes the
  currently active music event through existing render-to-simulation
  telemetry; the ordered audio-effect transport remains the only command path.

This slice does not implement the general environment background-music
scheduler, menu/credits music, music-frequency options, now-playing toasts,
per-biome audio attribute probing, or in-game configuration. It does not add
new Component translation/localization support; it preserves and presents the
same trusted run projection already used by chat and scoreboard paths. Boss
fog affects atmospheric cameras only, as in Vanilla; water and other special
fog environments remain governed by their own rules.

## Vanilla, wire, and integration contract

`ClientboundBossEventPacket` writes UUID, the operation enum ordinal, and one
of `ADD`, `REMOVE`, `UPDATE_PROGRESS`, `UPDATE_NAME`, `UPDATE_STYLE`, or
`UPDATE_PROPERTIES`. Add writes a trusted Component, float progress, color and
overlay enum ordinals, then one unsigned property byte; update-name writes the
same trusted Component codec. Bits `1`, `2`, and `4` mean darken screen, play
boss music, and create world fog. The existing strict enum and trailing-byte
checks remain authoritative while both name operations switch to the shared
`RichTextComponent` decoder.

`BossHealthOverlay` stores `LerpingBossEvent` values in a `LinkedHashMap` and
iterates that map for drawing and aggregate properties. `put` of an existing
UUID replaces the event without moving its insertion slot; removing then
adding appends it. A bbb ordered vector carries the UUID in each bar and uses
in-place replacement for the same semantics. Each add receives an instance
revision and each progress operation advances a per-instance progress revision
so native presentation can distinguish equal-valued updates and replacement
events without storing a clock in world state.

`LerpingBossEvent` initializes current and target progress to the add value.
On update it first samples the currently visible value, sets that as the new
start, records the new target and `Util.getMillis()`, then returns
`lerp(clamp(elapsed / 100 ms, 0, 1), start, target)`. `HudRenderRuntime` owns
the equivalent `Instant` state keyed by UUID and instance/progress revisions.
It emits replacement HUD state while any transition is active, even when the
published world snapshot is unchanged; add/replacement jumps immediately,
remove drops presentation state, and session-generation changes clear it.

The property queries fold `any` over all active bars. `darken_screen` drives
`GameRenderer.tick`: `+0.05` up to `1`, then `-0.0125` while positive, with
partial-tick interpolation into the lightmap. `create_world_fog` clamps
atmospheric start/end to at most `10`/`96`, sets sky/cloud end to the clamped
end, divides the End-flash addition by three, and passes `false` as
`LevelRenderer.renderLevel`'s `shouldRenderSky`. The existing bbb lightmap and
fog calculations stay authoritative; this slice adds the missing sky gate and
locks all three consumers together with focused tests.

`Minecraft.getSituationalMusic` selects `Musics.END_BOSS` only when the player
dimension key is `minecraft:the_end` and any bar requests music. That music is
`minecraft:music.dragon` with min/max delay zero and
`replaceCurrentMusic=true`; the next `MusicManager.tick` stops a different
active music event and starts dragon immediately, and starts it again after
natural completion while the condition remains true. When the flag clears,
the ordinary End background `Musics.END` can replace dragon because its
replace flag is true. The native boss-music selector runs once per raw client
tick, emits direct local music-category commands through `AudioEventSink`, uses
the world's deterministic sound-seed chain, and reads only the backend's
active music event id from `AudioCounters`. For custom End environment music,
dragon is stopped on flag clear only when the selected background entry says
it replaces the current event; starting that background entry remains the
general scheduler's responsibility.

Authoritative local sources are
`net/minecraft/network/protocol/game/ClientboundBossEventPacket.java`,
`net/minecraft/client/gui/components/BossHealthOverlay.java`,
`LerpingBossEvent.java`, `net/minecraft/client/Minecraft.java`,
`net/minecraft/client/sounds/MusicManager.java`,
`net/minecraft/sounds/Musics.java`,
`net/minecraft/client/renderer/GameRenderer.java`,
`LightmapRenderStateExtractor.java`,
`net/minecraft/client/renderer/fog/environment/AtmosphericFogEnvironment.java`,
and `net/minecraft/data/worldgen/DimensionTypes.java` in the local Vanilla
26.1 source tree.

## Serial stages and verification

1. Decode both name operations as rich Components; replace the sorted world
   map with insertion-ordered canonical bars carrying exact name projections
   and deterministic instance/progress revisions. Focused tests: protocol
   add/update-name rich NBT, invalid/trailing input and all operation shapes;
   world add/replace/remove/re-add order, exact names, equal-valued progress
   revisions, aggregate flags, serde round trips, and unknown updates. Because
   this changes protocol decode, the stage gets a full gate.
2. Add the native presentation cache, project styled name runs, bypass stable
   world reuse only during active 100 ms transitions, and suppress sky
   submission under boss fog. Focused tests: zero/half/full interpolation,
   interrupted and equal-target updates, replacement/removal/session reset,
   ordered rich projection, stable-frame updates, renderer styled boss text,
   fog clamps, End-flash division, and sky suppression. This core presentation
   milestone gets a full gate.
3. Expose active music identity from the audio backend, carry it in existing
   backend telemetry, and tick the End boss-music selector through the ordered
   simulation audio-effect path. Focused tests: backend active/stopped/natural
   identity, non-End no-op, immediate replacement, no duplicate start,
   completion restart, flag-clear replacement policy, deterministic seed, and
   effect ordering. This cross-thread audio milestone gets a full gate.
4. Mark the packet row complete, remove both finished boss-bar TODOs, append
   the milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `fea4bcc6a` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Rich-decode failures stop before world migration; ordered
state failures stop before presentation; interpolation or sky failures stop
before audio; backend telemetry or selector failures stop before coverage
closeout. Failures are fixed forward without rewriting history.
