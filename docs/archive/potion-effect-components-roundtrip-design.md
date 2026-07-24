# Potion effect ItemStack component family round-trip

Status: complete and fully integrated.

## Goal and completion contract

Close all six ItemStack coverage dimensions for Vanilla 26.1
`POTION_CONTENTS` (type 51) and `SUSPICIOUS_STEW_EFFECTS` (type 53) as one
effect-bearing component engineering slice.

Completion means both catalog descriptors own exact network encoders.
Potion contents preserve the optional built-in potion holder, optional raw
custom color, ordered recursive custom mob-effect instances, and optional
Java string. Suspicious stew preserves the ordered effect-holder/duration
entries. Missing canonical presence projections, count mismatches, invalid
holders or amplifiers, excessive recursive depth, oversized strings, and
over-budget collections fail before any patch bytes are appended.

Creative ItemStacks must preserve independent per-value delimiters. Existing
persistent dialog/SNBT, potion tint, potion and creative-stew tooltip,
item-model exact predicates, recipe projection, and authoritative mob-effect
state consumers provide the semantic evidence required before both ledger
rows become `complete`.

## Non-goals

- This slice does not encode `POTION_DURATION_SCALE`, writable books, or
  another ItemStack component.
- `POTION_DURATION_SCALE` remains a separate readiness problem: item tooltip
  projection does not yet apply an explicit scale and the item prototype
  boundary does not retain tipped-arrow/lingering-potion default scalar
  values. Network encoding alone would not close its observable semantics.
- It does not predict server-side consumption. The server remains
  authoritative for applying potion and stew effects; the client consumes the
  resulting mob-effect packets through the existing canonical state path.
- It does not expand the existing untrusted list budget. Vanilla's unbounded
  `ByteBufCodecs.list()` is deliberately capped at 4096 entries by this
  client, and the encoders exactly cover every canonical value its decoders
  admit.
- It does not expand the existing recursive effect budget of 16 nested
  `hidden_effect` links.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns exact network
  encoding, built-in holder validation, collection and recursive-effect
  validation, descriptor registration, and atomic output.
- `bbb-protocol::packets::data_components` retains typed decoding, canonical
  summaries, patch preflight, and delimited framing.
- `bbb-world` retains persistent dialog projection, potion recipe output, and
  canonical mob-effect state.
- `bbb-item-model` retains potion tint, potion/stew tooltip, and exact
  component predicates; `bbb-native` retains persistent SNBT projection.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- Both `DataComponents` registrations are persistent, network-synchronized
  with their named StreamCodec, and cache encoding.
- `PotionContents.STREAM_CODEC` writes an optional built-in potion holder, an
  optional raw i32 custom color, a VarInt-length list of complete
  `MobEffectInstance` values, and an optional UTF-8 Java string.
- `MobEffectInstance.STREAM_CODEC` writes the built-in effect holder followed
  by recursive details: amplifier VarInt, duration VarInt, ambient,
  show-particles, show-icon, and optional hidden details.
- `SuspiciousStewEffects.STREAM_CODEC` writes a VarInt-length list whose
  entries are a built-in mob-effect holder and raw duration VarInt.
- `MobEffectInstance` clamps incoming amplifiers to `0..=255`; its persistent
  Codec uses the same unsigned-byte domain.
- `PotionContents.getColor`, `PotionContents.addToTooltip`,
  `PotionContents.onConsume`, and `SuspiciousStewEffects.addToTooltip` /
  `onConsume` consume the retained fields.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` length-prefixes each complete
  component value independently.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/{DataComponents,DataComponentPatch}.java`,
`world/item/alchemy/{Potion,PotionContents}.java`,
`world/item/component/SuspiciousStewEffects.java`,
`world/effect/MobEffectInstance.java`, and
`network/codec/ByteBufCodecs.java`.

## Serial stages and validation

1. Add exact descriptor encoders and focused protocol tests covering absent
   and present optional fields, raw signed colors/durations, recursive hidden
   details, collection and depth boundaries, Java UTF-16 string boundaries,
   invalid holders/amplifiers, count alignment, and atomic failure. Safe stop:
   descriptor tests pass with no ledger claim.
2. Add canonical patch preflight, independent delimiters, and exact creative
   packet tests. Re-run persistent dialog/SNBT, potion tint/tooltip,
   creative-stew tooltip, recipe, and exact predicate tests. Safe stop: types
   51 and 53 are the only newly encodable values.
3. Close all twelve ledger cells, narrow the creative TODO, archive this
   contract, and add the engineering index entry. Run affected focused tests,
   generator checks, and every per-commit check.

This is protocol/component high risk. The preceding firework checkpoint
`c2e8c2619` passed the full gate before implementation. Commit the complete
slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward before
the next slice.
