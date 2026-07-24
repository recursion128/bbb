# Weapon-interaction ItemStack component family round-trip

## Goal and completion contract

Close all six ItemStack coverage dimensions for Vanilla 26.1
`PIERCING_WEAPON` (type 38), `KINETIC_WEAPON` (type 39), and
`SWING_ANIMATION` (type 40) as one adjacent weapon-interaction engineering
slice.

Completion means all three catalog descriptors own exact network encoders.
Piercing weapons preserve both booleans and both optional sound holders.
Kinetic weapons preserve both tick fields, all three optional condition
records, raw float bits, and both optional sound holders. Swing animation
preserves the generated animation type id and duration. Malformed sound
unions, missing canonical summaries, and stale or unrelated projections fail
before any patch bytes are appended.

Creative ItemStacks must encode any combination through independent
per-value delimiters. Existing persistent dialog/SNBT, piercing attack input,
kinetic spear use/feedback, and first-/third-person swing animation consumers
must provide named semantic evidence before all three ledger rows become
`complete`.

## Non-goals

- This slice does not encode `ATTACK_RANGE`, `WEAPON`, another component, or
  the complete creative-slot component surface.
- It does not reproduce server-authoritative piercing/kinetic damage,
  knockback, dismount, or hit selection. The native client sends Vanilla's
  STAB player action and observes resulting entity state.
- It does not apply persistent Codec ranges to network integers or floats.
  Vanilla stream codecs transmit VarInts and raw float bits; persistent SNBT
  separately enforces non-negative ticks and positive swing duration.
- It does not hand-copy animation ordinals. The encoder uses the generated
  `SwingAnimationTypeSummary` mapping already shared with decoding; unknown
  incoming ids retain Vanilla's `OutOfBoundsStrategy.ZERO` normalization.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns the three exact
  network encoders, optional condition records, optional sound-holder
  validation, descriptor registration, and atomic output.
- `bbb-protocol::packets::data_components` retains typed decoding, canonical
  summaries, patch preflight, and delimited framing.
- `bbb-world` retains persistent dialog Codec projection and item prototype
  defaults used by native inventory and render consumers.
- `bbb-native` retains persistent SNBT rules, piercing left-click routing,
  kinetic spear use/feedback, and first-person swing selection;
  `bbb-renderer` retains third-person WHACK/NONE/STAB presentation.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- The three `DataComponents` registrations are persistent,
  network-synchronized, and cache encoded with their named Codec/StreamCodec
  pairs.
- `PiercingWeapon.STREAM_CODEC` writes `dealsKnockback`, `dismounts`, optional
  sound, then optional hit sound.
- `KineticWeapon.STREAM_CODEC` writes contact cooldown and delay VarInts,
  optional dismount/knockback/damage conditions, forward movement and damage
  multiplier floats, optional sound, then optional hit sound. Each condition
  writes max-duration VarInt, min-speed float, and min-relative-speed float.
- `SwingAnimation.STREAM_CODEC` writes the generated `SwingAnimationType`
  VarInt followed by duration VarInt.
- Network integers and floats have no StreamCodec range wrapper. Persistent
  Codecs require non-negative kinetic ticks/condition duration and positive
  swing duration while retaining unrestricted Codec floats.
- `Minecraft.startAttack` sends the STAB action for a piercing weapon;
  `Item`, `LivingEntity`, and `SpearAnimations` consume kinetic weapons;
  avatar and humanoid renderers consume swing animation.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps each complete component
  value in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/PiercingWeapon.java`,
`world/item/component/KineticWeapon.java`,
`world/item/component/SwingAnimation.java`, `world/item/SwingAnimationType.java`,
`client/Minecraft.java`, `client/model/effects/SpearAnimations.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Add exact descriptor encoders for the three records and focused protocol
   tests covering optional combinations, direct/registry sound holders,
   special float bits, generated animation ids, raw negative network integers,
   malformed unions, missing values, and atomic failure. Safe stop: descriptor
   tests pass with no ledger claim.
2. Add canonical patch preflight, per-value delimiter, and exact creative
   packet tests. Re-run piercing input, kinetic first-person, and swing
   first-/third-person focused tests. Safe stop: types 38 through 40 are the
   only newly encodable values.
3. Audit persistent dialog/SNBT evidence; close all eighteen ledger cells,
   narrow the creative TODO, and archive this contract. Run affected focused
   tests, generator checks, and every per-commit check.

This is protocol/component high risk. The preceding consume-effect checkpoint
`25bcd2e72` passed the full gate before implementation. Commit the complete
slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward before the
next slice.
