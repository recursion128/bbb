# Explosion packet coverage closure

## Goal and completion contract

Close the three remaining coverage cells for `ClientboundExplodePacket`:
decode, canonical apply, and runtime side effect. The packet must retain its
registry-dispatched primary particle, sound-event holder, and weighted
secondary particle list; the world owner must apply local-player knockback and
record one typed diagnostic event; the native runtime must play the positioned
explosion sound, spawn the primary particle immediately, and reproduce
`ClientExplosionTracker` on the next client tick.

Completion means all five packet dimensions are `complete`, no raw explosion
tail remains, malformed counts and weights cannot cause unbounded allocation or
integer overflow, the finished unsupported-feature todo is removed, generated
packet coverage agrees with its override source, and the full plus playable
gates pass at the milestone head.

## Scope and non-goals

- `bbb-protocol` owns the exact wire order, built-in particle dispatch,
  sound-event holder, bounded weighted list, non-negative weights, and checked
  total weight.
- `bbb-world` owns the typed last-event projection, finite local-player
  knockback, deterministic explosion sound state, and the single
  `PlayApplyEffects` callback.
- `bbb-native` owns immediate primary-particle submission and the ephemeral
  secondary-particle tracker. It reads block/biome context from the canonical
  world but does not publish its queue in world snapshots.
- Existing audio holder resolution and particle option resolution remain the
  only registry/resource interpretation paths.

This slice does not implement client-authoritative explosion damage, fire, or
block destruction; server packets continue to own those canonical changes. It
does not add renderer-specific explosion particles, change the particle status
option, or refactor unrelated level-event particle handling.

## Vanilla and integration contract

- `ClientboundExplodePacket.STREAM_CODEC` carries center, radius, block count,
  optional player knockback, `ParticleTypes.STREAM_CODEC`,
  `SoundEvent.STREAM_CODEC`, then
  `WeightedList.streamCodec(ExplosionParticleInfo.STREAM_CODEC)` in that order.
  Each weighted row is particle options, scaling, speed, then a non-negative
  VarInt weight. bbb applies a 65,536-row hostile-input allocation cap and
  rejects a total weight above `i32::MAX`.
- `ClientPacketListener.handleExplosion` plays the packet sound at the center
  in the blocks category with volume `4.0`, pitch
  `(1 + (nextFloat - nextFloat) * 0.2) * 0.7`, and no distance delay; adds the
  primary particle with velocity `(1, 0, 0)`; tracks the secondary list; then
  adds optional knockback to the local player.
- `WorldStore::apply_play_packet` preserves that order. It uses the shared
  `LevelEventSoundRandomState` for the two pitch draws and sound seed, records
  the positioned sound through the existing audio state, applies finite
  knockback, and sends the accepted typed explosion to one runtime callback.
  Sink-less callers consume the identical random sequence.
- `ClientExplosionTracker.tick` clears pending work unless particle status is
  `ALL`. Otherwise it chooses explosions by block count, caps the combined
  output at 512, samples a cubic-radius position and only emits in air, then
  chooses an `ExplosionParticleInfo` by weight and applies its scaling and speed.
  Native drops non-finite or non-positive visual-only tracker inputs rather than
  allowing malformed server data to reach renderer coordinates.

Authoritative sources are
`net/minecraft/network/protocol/game/ClientboundExplodePacket.java`,
`net/minecraft/client/multiplayer/ClientPacketListener.java`'s
`handleExplosion`,
`net/minecraft/core/particles/ExplosionParticleInfo.java`,
`net/minecraft/util/random/WeightedList.java`, and
`net/minecraft/client/multiplayer/ClientExplosionTracker.java` in the local
vanilla 26.1 source tree.

## Serial stages and verification

1. Replace the raw tail with bounded typed protocol fields, reuse the existing
   sound-holder and particle-option decoders, and update protocol/net fixtures.
   Focused tests: explosion decoder tests, protocol packet coverage tests, and
   `bbb-protocol`/`bbb-net` non-test checks.
2. Extend the typed world event and `PlayApplyEffects` contract; apply
   knockback, audio state, primary particle, and tracker registration through
   the shared packet path. Focused tests: explosion world/play-apply tests and
   native dispatcher effect-order tests.
3. Add the native ephemeral tracker with deterministic weighted sampling,
   particle-status clearing, air filtering, scaling/speed, and the 512 cap.
   Focused tests: explosion particle-runtime tests and full `bbb-native` tests.
4. Mark the coverage row complete, remove the finished todo, append the
   milestone index, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `ef48ac95c` is the pre-start checkpoint. Each stage is a clean,
buildable commit. Decoder/fixture failures stop in stage 1; random-order or
side-effect routing failures stop in stage 2; tracker sampling failures stop in
stage 3. Failures are fixed forward without rewriting history.
