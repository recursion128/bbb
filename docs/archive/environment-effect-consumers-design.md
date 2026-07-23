# Environment effect consumers closure contract

## Goal and completion contract

Close the remaining Vanilla 26.1 `EnvironmentAttributeSystem` consumers over
the existing position-sampled world projection. The native client must run the
general delayed music scheduler, biome ambient loop/mood/additions handler, and
the 667-pair client animate loop for synchronized ambient particles plus the
firefly-bush and creaking-heart environment-gated local effects.

Completion means every consumer samples the exact Vanilla position and biome
mode; raw-client versus running-level tick gates and side-effect order match the
official client; music replacement, delay, natural completion, gain fading,
and End boss priority share one runtime owner; ambient loops cross-fade over 40
ticks and recover after resource reload; mood and additions use their
synchronized holder/direct sound data; ambient particle probabilities and
positions use the client-level random stream; firefly and creaking checks use
the sampled block position; missing chunks, registry holders, particle
providers, and sound assets retain diagnostics without panics or bootstrap
ordinal substitution; `ChunksBiomes` replacement changes all applicable
effects on the next tick; and the exact environment TODO and remaining
`CLIENTBOUND_CHUNKS_BIOMES` coverage cells can be closed.

## Scope, ownership, and frozen integration surface

- `bbb-world` remains the only owner of synchronized environment values,
  direct/Gaussian position sampling, chunk/block/light/heightmap queries, sound
  holder data, and deterministic sound seeds. No backend handle or native
  scheduler state moves into `WorldStore`.
- `bbb-native::music_runtime` becomes the single owner of ordinary and End-boss
  `MusicManager` state. `MusicRuntime` replaces `BossMusicRuntime`; callers use
  `advance_for_ticks(world, connection_generation, raw_client_ticks, audio)`.
- `bbb-native::environment_audio_runtime` owns the
  `BiomeAmbientSoundsHandler` projection: previous loop identity, per-loop
  40-tick fade, moodiness, and its deterministic client-local random cursor.
  It advances on running local-player ticks and emits only audio effects.
- `bbb-native::environment_animate_runtime` owns the client-level and
  animate-random cursors and the 667 direct-position sample pairs per running
  level tick. It emits bounded particle batches and one-shot audio effects; it
  never mutates authoritative blocks.
- `bbb-native::AudioEventSink` gains
  `set_music_gain(gain: f32)` and
  `sync_ambient_loops(loops: &[AmbientLoopSoundState])`.
  `AmbientLoopSoundState`, exported by `bbb-world`, contains the exact
  `ClientSoundEventHolder`, the clamped current gain, and a stable selection
  seed. The simulation effect collector preserves these calls in FIFO order.
- `bbb-audio::AudioCommand` gains typed music-gain and ambient-loop sync
  commands. Kira owns the corresponding handles; every sync is an idempotent
  desired-state replacement so the next tick restores loops after resource
  reload and stops loops omitted after level/session replacement.
- `bbb-native::ParticleEventSink` gains one batched environment-particle entry
  accepting exact `ClientParticleOptions` and positions. Existing particle
  catalog/provider resolution remains authoritative; unsupported complex
  options are diagnosed and dropped rather than coerced to a simple particle.

This slice does not implement other block/fluid `animateTick` behaviors, marker
particles, fluid drip particles, configurable music-frequency UI, a now-playing
toast, authenticated services, server-owned creaking-heart state transitions
or spawning, or the separate complex default-dripstone option TODO. It does not
add configuration UI. The ambient animate loop preserves the shared
client-level draws that determine sampled positions and the supported
environment effects, while unrelated block/fluid providers retain their own
open ledger items.

## Vanilla facts, tick order, and fallbacks

`Minecraft.tick` advances `MusicManager` once per raw client tick after
`GameRenderer.tick` and before `SoundManager.tick`, even when the level is
paused. `LocalPlayer.tick` advances its ambient sound handlers only when the
client has loaded; the biome handler therefore follows running local-player
ticks. `ClientLevel.animateTick` runs only for a non-paused, normally running
level, after `ClientLevel.tick`, and before `ParticleEngine.tick`.

`Minecraft.getSituationalMusic` selects screen music first, then End boss
music, then the camera-Gaussian `BACKGROUND_MUSIC` entry with underwater before
creative before default, and otherwise menu music. bbb has no in-game screen
music owner, so gameplay selection covers End boss and synchronized background
music; no level remains silent rather than inventing menu state. The general
runtime starts with delay 100, clamps candidate delay by the selected
default-frequency maximum of 24,000 ticks, applies replacement before natural
completion, decrements with the Java post-decrement condition, and sets the
post-start delay to `i32::MAX`. Gain follows `MusicManager.fadePlaying`: rising
gain adds `clamp(currentGain, 0.0005, 0.005)`, falling gain blends
`0.03 * target + 0.97 * current`; gain at or below `0.0001` stops the music.
All random inclusive ranges consume one runtime-owned deterministic cursor.

`BiomeAmbientSoundsHandler` directly samples `AMBIENT_SOUNDS` at
`LocalPlayer.position()`. A loop identity change fades every old loop out and
fades the selected loop in over 40 ticks, reusing a still-fading identical
holder. Additions test each ordered entry's `tickChance` and play a relative
ambient one-shot. Mood samples a cube of side
`blockSearchExtent * 2 + 1` around player X/Z and eye Y. Sky light reduces
moodiness by `sky / 15 * 0.001`; otherwise block light changes it by
`-(block - 1) / tickDelay`. At one it places the sound beyond the sampled block
by the synchronized offset, plays it positionally, and resets moodiness; values
below one clamp to zero. Missing loaded light/chunk data skips that tick's mood
sample without fabricating darkness.

`ClientLevel.animateTick` performs 667 iterations, sampling radius 16 then 32.
It creates one fresh `animateRandom` per tick and shares it across all 1,334
block/fluid calls in that tick.
Each coordinate is center plus `nextInt(radius) - nextInt(radius)`.
`AMBIENT_PARTICLES` is directly sampled at that block position only when the
known collision shape is not a full block; ordered entries test
`nextFloat() <= probability` and successful entries consume three
`nextDouble()` position draws. Firefly-bush sound uses the separate animate
random, a one-in-30 check, direct `FIREFLY_BUSH_SOUNDS`, and
`MOTION_BLOCKING_NO_LEAVES <= y`. Its firefly spawn uses maximum local raw
brightness 13, probability 0.7, horizontal range 10, and vertical range 5.
Creaking-heart idle sound requires direct `CREAKING_ACTIVE`, a non-uprooted
state, a one-in-16 check, and pale-oak logs in all six adjacent positions.
Successful firefly and creaking local sounds consume their seed from the shared
client-level random stream at the Vanilla `playLocalSound` call point.
Unknown block states or unavailable height/light/collision samples suppress
only the affected local effect.

Authoritative local sources are
`net/minecraft/client/Minecraft.java`,
`net/minecraft/client/sounds/MusicManager.java`,
`net/minecraft/client/player/LocalPlayer.java`,
`net/minecraft/client/resources/sounds/BiomeAmbientSoundsHandler.java`,
`net/minecraft/client/multiplayer/ClientLevel.java`,
`net/minecraft/world/level/block/FireflyBushBlock.java`,
`net/minecraft/world/level/block/CreakingHeartBlock.java`,
`net/minecraft/world/attribute/EnvironmentAttributeSystem.java`, and
`EnvironmentAttributes.java`.

## Serial stages and verification

1. Add typed music-gain and desired ambient-loop audio commands through
   `bbb-audio`, native resolution, the audio effect collector, and backend
   handle lifecycle. Focused tests cover command replay order, idempotent loop
   sync, fade gain updates, omission stop, natural completion, and reload
   recovery. Run focused `bbb-audio` and `bbb-native audio_runtime` tests.
2. Replace the boss-only runtime with the full general music scheduler while
   retaining End-boss precedence and telemetry-backed retry behavior. Focused
   tests cover all selection priorities, inclusive delays, replacement,
   completion, gain curves, connection reset, and FIFO order. Run
   `scripts/cargo-dev.sh test -p bbb-native music_runtime`.
3. Implement biome ambient loop/mood/additions over direct player samples,
   loaded light queries, and desired-loop sync. Focused tests cover the
   40-tick cross-fade, ordered random consumption, mood recovery/trigger
   position, missing-data fallback, holder reorder, and biome replacement.
   Run `scripts/cargo-dev.sh test -p bbb-native environment_audio`.
4. Run the checkpoint full gate after the contract plus three audio stages.
   Then implement the bounded 667-pair ambient/firefly/creaking animate runtime
   and exact particle option bridge. Focused tests cover pair order, position
   and probability draws, collision suppression, height/light/log gates,
   missing providers, and `ChunksBiomes` replacement. Run
   `scripts/cargo-dev.sh test -p bbb-native environment_animate`.
5. Remove the completed environment TODO, mark the applicable
   `CLIENTBOUND_CHUNKS_BIOMES` cells complete with exact semantic evidence,
   append the milestone index, archive this contract, review the integrated
   diff, and run `scripts/cargo-dev.sh gate`,
   `scripts/playable-gate.sh live`, `scripts/playable-gate.sh auto`, and the P0
   audit.

The fully gated `cd720ca4e` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Audio-command failures stop before scheduler replacement;
music failures stop before ambient handler work; ambient-audio failures stop
before the high-volume animate loop; animate provider/query failures stop
before coverage closure. Failures are fixed forward without rewriting history.
