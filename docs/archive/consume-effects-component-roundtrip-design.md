# Consume-effect ItemStack component family round-trip

## Goal and completion contract

Close all six ItemStack component coverage dimensions for Vanilla 26.1
`CONSUMABLE` (type 24) and `DEATH_PROTECTION` (type 36) as one shared
consume-effect codec engineering slice.

Completion means both catalog descriptors own their exact network encoders.
`CONSUMABLE` must preserve consume-seconds bits, item-use animation, required
sound holder, particle flag, and ordered effects. `DEATH_PROTECTION` must
preserve its ordered effects. Every effect must retain its generated built-in
type id and exact branch payload: recursive mob-effect instances and
probability, mob-effect holder set, unit, teleport diameter, or sound holder.
All lists and recursion remain bounded; malformed unions, stale counts,
invalid holders, and noncanonical summaries fail before any patch bytes are
appended.

Creative ItemStacks must encode either or both components through the existing
per-value delimiter. Existing persistent dialog publication and SNBT,
first-person consumable animation/duration, component-presence item-model
selection, and server-authored client effect state must provide named evidence
before both ledger rows become `complete`.

## Non-goals

- This slice does not encode `FOOD`, `USE_REMAINDER`, another component, or the
  complete creative-slot component surface.
- It does not execute consume or death-protection effects client-authoritatively.
  Vanilla applies these effects on the server; the client observes the
  resulting sound, position, health, and mob-effect packets through their
  existing owners.
- It does not apply persistent Codec ranges to network floats. Vanilla stream
  codecs transmit raw bits for consume seconds, probability, teleport
  diameter, and mob-effect fields; persistent SNBT retains its own non-negative,
  positive, unsigned-byte, and 0-through-1 validation.
- It does not allow custom consume-effect type ids. The dispatch registry is
  built-in and the generated 26.1 catalog defines the only five branches.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns the two exact network
  encoders, shared polymorphic effect encoding, mob-effect recursion,
  holder/sound union validation, descriptor registration, and atomic output.
- `bbb-protocol::packets::data_components` owns bounded decoding, generated
  consume-effect type dispatch, canonical summaries, patch preflight, and
  delimited framing.
- `bbb-world` retains persistent dialog Codec decoding and projection of all
  five effect variants with session registry context.
- `bbb-native` retains persistent SNBT range/default validation and
  first-person consume animation/duration. `bbb-item-model` owns generic
  component-presence selection and consumable use-duration projection.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.CONSUMABLE` and `DEATH_PROTECTION` are persistent,
  network-synchronized, and cache encoded with their respective Codec and
  StreamCodec pairs.
- `Consumable.STREAM_CODEC` writes, in order: consume-seconds float,
  `ItemUseAnimation` VarInt, sound-event holder, particle boolean, and an
  ordered `ConsumeEffect.STREAM_CODEC` list. `DeathProtection.STREAM_CODEC`
  writes only the same ordered effect list.
- `ConsumeEffect.STREAM_CODEC` dispatches through the built-in
  `consume_effect_type` registry. Vanilla registration order is
  apply-effects 0, remove-effects 1, clear-all-effects 2, teleport-randomly 3,
  and play-sound 4.
- Apply-effects writes a list of `MobEffectInstance.STREAM_CODEC` followed by a
  probability float. A mob-effect instance writes a holder id and recursively
  writes amplifier, duration, ambient, particles, icon, and optional hidden
  details. Construction clamps every amplifier to 0 through 255.
- Remove-effects writes a mob-effect holder set. Clear-all has no payload.
  Teleport-randomly writes one float. Play-sound writes one sound-event holder.
- Persistent defaults and ranges are owned by `Consumable.CODEC`,
  `DeathProtection.CODEC`, and each effect MapCodec. They do not constrain the
  raw network float codecs.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps each complete component
  value in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/Consumable.java`,
`world/item/component/DeathProtection.java`,
`world/item/consume_effects/*.java`, `world/effect/MobEffectInstance.java`,
`sounds/SoundEvent.java`, `network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Add shared exact consume-effect and recursive mob-effect encoders, then
   register type 24 and 36. Focused protocol tests cover all five branches,
   registry/direct sounds, tag/direct holder sets, raw special float bits,
   recursive details, empty and maximum lists, depth/union/id/count failures,
   and atomic rejection. Safe stop: protocol tests pass with no ledger claim.
2. Add patch-preflight, per-value delimiter, and exact creative packet tests
   for both components. Add a focused item-model presence test for the
   server-authoritative death-protection component and re-run existing
   first-person consumable tests. Safe stop: types 24 and 36 are the only newly
   encodable values.
3. Audit persistent dialog/SNBT and client-effect evidence; close all twelve
   ledger cells, narrow the creative TODO, and archive this contract. Run
   affected focused tests, generator checks, and every per-commit check.

This is protocol/component high risk. The preceding blocks-attacks checkpoint
`50b1dbb29` passed the full gate before implementation. Commit the complete
slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward before the
next slice.
