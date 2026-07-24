# Firework ItemStack component family round-trip

Status: complete and fully integrated.

## Goal and completion contract

Close all six ItemStack coverage dimensions for Vanilla 26.1
`FIREWORK_EXPLOSION` (type 68) and `FIREWORKS` (type 69) as one nested firework
engineering slice.

Completion means both catalog descriptors own exact network encoders.
Standalone firework explosions preserve the generated shape id, every raw
32-bit color and fade color, trail, and twinkle. Fireworks preserve the raw
flight-duration VarInt and the ordered list of complete nested explosions.
Missing canonical presence fields, mismatched declared counts, over-budget
color lists, and oversized explosion lists fail before any patch bytes are
appended.

Creative ItemStacks must preserve independent per-value delimiters. Existing
persistent dialog/SNBT, firework item tint and tooltip, rocket entity item
projection, entity-event particle, shape/color/fade/trail/twinkle, and sound
consumers provide the semantic evidence required before both ledger rows
become `complete`.

## Non-goals

- This slice does not encode `LODESTONE_TRACKER`, `PROFILE`, or another
  ItemStack component.
- It does not alter rocket flight physics, explosion damage, particle
  geometry, sound selection, item tint, or tooltip grouping.
- It does not impose the persistent unsigned-byte range on the network
  flight-duration VarInt. Vanilla `Fireworks.STREAM_CODEC` transmits a raw
  VarInt; persistent SNBT separately accepts only `0..=255`.
- It does not expand the existing untrusted color-list budget.
  `ByteBufCodecs.list()` permits up to `Integer.MAX_VALUE`, while this client
  intentionally admits at most 4096 colors or fade colors before allocation.
  The encoder exactly covers every canonical value the decoder admits.
- It does not hand-copy shape ordinals. The encoder uses the existing
  `FireworkExplosionShapeSummary` id mapping shared with decoding; unknown
  incoming ids retain Vanilla's `OutOfBoundsStrategy.ZERO` normalization.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns both exact network
  encoders, shared explosion encoding, collection validation, descriptor
  registration, and atomic output.
- `bbb-protocol::packets::data_components` retains typed decoding, generated
  shape normalization, canonical summaries, patch preflight, and delimited
  framing.
- `bbb-protocol::packets::serverbound` owns the persistent unsigned-byte
  flight-duration check for creative untrusted ItemStacks while leaving the
  component StreamCodec's raw VarInt domain intact.
- `bbb-world` retains persistent dialog projection, rocket entity ItemStack
  state, and entity-event explosion projection.
- `bbb-item-model` retains firework tint and tooltip presentation;
  `bbb-native` retains persistent SNBT, rocket rendering, deterministic
  particles, colors/fades, trail/twinkle, and sound presentation.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- Both `DataComponents` registrations are persistent, network-synchronized
  with their named StreamCodec, and cache encoding.
- `FireworkExplosion.STREAM_CODEC` writes shape VarInt, a VarInt-length list
  of big-endian i32 colors, a second list of fade colors, trail boolean, then
  twinkle boolean.
- `FireworkExplosion.Shape.STREAM_CODEC` uses the generated ids 0 through 4
  and normalizes an out-of-range incoming id to `SMALL_BALL`.
- `Fireworks.STREAM_CODEC` writes flight-duration VarInt followed by at most
  256 complete `FireworkExplosion` values.
- The persistent fireworks Codec restricts flight duration to an unsigned
  byte and both persistent/network paths restrict the explosion count to 256.
- `FireworkExplosion.addToTooltip`, `Fireworks.addToTooltip`,
  `FireworkRocketEntity.getExplosions`, and `FireworkParticles.Starter`
  consume the retained fields.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` length-prefixes each complete
  component value independently.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`core/component/DataComponentPatch.java`,
`network/codec/ByteBufCodecs.java`,
`world/item/component/{FireworkExplosion,Fireworks}.java`,
`world/entity/projectile/FireworkRocketEntity.java`, and
`client/particle/FireworkParticles.java`.

## Serial stages and validation

1. Add exact descriptor encoders and focused protocol tests covering all five
   shapes, empty/non-empty colors and fades, raw signed i32 values, raw
   negative flight duration, 256 explosions, collection overflow, missing or
   mismatched projections, and atomic failure. Safe stop: descriptor tests
   pass with no ledger claim.
2. Add canonical patch preflight, independent delimiters, the creative
   persistent flight-duration range, and exact creative packet tests. Re-run
   persistent dialog/SNBT, item tint/tooltip, rocket state, entity-event
   particle, and native firework resolver tests. Safe stop: types 68 and 69
   are the only newly encodable values.
3. Close all twelve ledger cells, narrow the creative TODO, archive this
   contract, and add the engineering index entry. Run affected focused tests,
   generator checks, and every per-commit check.

This is protocol/component high risk. The preceding armor-trim checkpoint
`e9af28e5e` passed the full gate before implementation. Commit the complete
slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward before
the next slice.
