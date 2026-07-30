# Heart Health Blink Presentation Engineering Contract

Status: active from 2026-07-29.

## Goal and completion

Close the active player-heart damage/heal blink gap with the exact observable
Vanilla 26.1 presentation state machine:

- retain the previous displayed health for the one-second wall-clock hold;
- arm a 20-tick damage flash or 10-tick heal flash only while the local player
  has positive client-side damage invulnerability time;
- compute the current frame's blink phase before applying that frame's health
  edge; and
- draw blinking containers and old-health fills beneath the current normal
  health fill in Vanilla order.

Completion requires deterministic native tests for initial projection,
damage/heal edges, the pre-edge phase rule, the three-tick blink cadence,
strict one-second expiry, and the invulnerability gate; renderer layout tests
for damage/heal draw strata and max-health expansion; deterministic offscreen
GPU evidence that blinking old fills and normal current fills consume distinct
resident sprites in the required order; removal of the active ledger item;
archival of this contract; a concise milestone index entry; and a passing full
gate at the final head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/gui/Gui.java`,
  especially `extractPlayerHealth`, `extractHearts`, and `extractHeart`; and
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/entity/LivingEntity.java`,
  especially the client-side `invulnerableTime` decrement in `baseTick`.

`Gui.extractPlayerHealth` ceils the current health. It computes
`healthBlinkTime > tickCount && (healthBlinkTime - tickCount) / 3 % 2 == 1`
before observing the current health edge. A decrease with positive
`invulnerableTime` records the wall-clock timestamp and arms
`healthBlinkTime = tickCount + 20`; an increase under the same gate arms
`tickCount + 10`. The displayed health is replaced with current health only
when `timeMillis - lastHealthTime > 1000`, not at equality. It then stores the
current health as `lastHealth`.

The row maximum is the maximum of the MAX_HEALTH attribute, old displayed
health, and current health. For every descending container index,
`extractHearts` emits the container using the frame blink flag, then a
non-blinking absorption fill, then the blinking old-health fill when active,
then the non-blinking current-health fill. Thus lost health remains visible as
a blinking old fill, while overlapping current health covers the old fill.

The existing `bbb-world` DamageEvent path already owns the required canonical
fact: `LivingEntity.handleDamageEvent` sets
`EntityDamage.invulnerable_time = 20`, ordinary client living ticks decrement
it, and `WorldStore::entity_damage_invulnerable_time` exposes it. This slice
does not infer invulnerability from wall-clock time or add protocol state.

## Ownership and frozen interfaces

- `bbb-world` remains the canonical owner of health, MAX_HEALTH, absorption,
  effects, hardcore mode, local-player identity, and client-side damage
  invulnerability ticks. It receives no presentation clock or blink state.
- `bbb-native::HudRenderRuntime` becomes the sole owner of the process-local
  `lastHealth`, `displayHealth`, `lastHealthTime`, and `healthBlinkTime`
  equivalents. A deterministic helper accepts an explicit `Instant`, client
  tick, ceiled current health, and canonical invulnerability time and returns
  the old displayed health plus this frame's blink flag.
- The initial helper state follows Java field defaults (`lastHealth = 0`,
  `displayHealth = 0`, no prior timestamp). A missing timestamp is treated as
  an expired hold for display catch-up. Missing local-player health produces no
  renderer projection and does not fabricate a health edge.
- Native projects `HudPlayerHealth` with raw current health, integer displayed
  health, and the frame blink flag. All other existing heart inputs remain
  unchanged. The invulnerability query uses only the current local-player
  entity id; missing canonical damage state falls back to zero.
- `bbb-renderer` remains stateless. Its heart layout expands the row maximum
  with displayed health and adds a per-instance blinking bit. Sprite lookup
  consumes that bit through the existing complete normal/hardcore,
  full/half, and blinking/non-blinking heart residency tables.

## Bounds and fallback rules

1. The blink deadline uses saturating tick addition and the phase test is false
   at or beyond the deadline. A deadline created by the current health edge
   cannot affect that same frame because phase extraction precedes edge
   handling.
2. The one-second hold uses monotonic `Instant` elapsed time and strict
   `> 1000 ms`. A backward or identical timestamp does not expire the hold.
3. Health values are ceiled at the native/renderer boundary exactly as
   Vanilla does. The existing renderer setter continues to reject non-finite
   raw health, maximum health, or absorption.
4. Absorption never uses blinking sprites. Regeneration lift and low-health
   shake continue to use current health and the existing deterministic client
   tick behavior.
5. Missing blinking sprite residency retains the existing bounded omission
   behavior for that draw; no substitute atlas, handwritten resource table, or
   renderer-owned state machine is added.

## Non-goals

- This slice does not change DamageEvent decode, damage acceptance, health
  synchronization, damage audio, hurt overlays, camera tilt, entity animation,
  or client tick scheduling.
- It does not add server combat calculation, infer health loss from
  DamageEvent, or synthesize invulnerability when canonical entity state is
  absent.
- It does not change food, armor, air, vehicle-health, accessibility,
  now-playing, or any other HUD feature.
- It does not add in-game configuration or refactor the general HUD command,
  atlas, or simulation-publication architecture.

## Serial stages and verification

1. **Native presentation state.** Add the deterministic wall-clock/tick state
   machine to `HudRenderRuntime`, query the current local-player
   invulnerability time, and project displayed health plus the frame blink bit.
   Run
   `scripts/cargo-dev.sh test -p bbb-native heart` and
   `scripts/cargo-dev.sh test -p bbb-world damage_invulnerable`.
2. **Renderer consumption.** Add ordered blinking instances and consume their
   typed sprite variant. Lock damage/heal order, row expansion, absorption
   exclusion, and offscreen pixels. Run
   `scripts/cargo-dev.sh test -p bbb-renderer heart`.
3. **Closeout.** Re-run all focused families, remove the ledger item, archive
   this contract, append the milestone index, run the standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`b9de99e8bf00565b42b93cab443a7d1f184f8e5e`. The final full gate is the
engineering checkpoint. No playable gate is required because this slice
changes no input, world mutation, packet, or control-command path.

## Failure localization and safe stops

Native failures compare ceiled current health, canonical invulnerability time,
prior displayed health, monotonic elapsed time, tick deadline, and the
pre-edge phase result before renderer code. Renderer failures compare generated
instance order and per-instance blinking identity before atlas residency,
command order, and pixels.

The clean contract commit and the buildable native-state commit are safe stops.
Any focused or gate failure is fixed forward before another slice; history is
not rewritten.
