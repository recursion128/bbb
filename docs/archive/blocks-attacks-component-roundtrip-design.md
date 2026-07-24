# Blocks attacks ItemStack component round-trip

## Goal and completion contract

Close all six ItemStack component coverage dimensions for Vanilla 26.1
`BLOCKS_ATTACKS` (type 37).

Completion means its catalog descriptor owns the exact
`BlocksAttacks.STREAM_CODEC` encoder. The two top-level floats, ordered damage
reductions, item-damage function, optional damage-type holder sets, and both
optional sound holders must round-trip without loss. Network float values are
preserved bit-for-bit because the stream codec does not apply the persistent
Codec's range predicates. Lists remain bounded by the native client safety
budget; holder tags, holder ids, direct sound ids, malformed holder unions, and
misowned component values are validated before any patch bytes are appended.

Creative ItemStacks must encode the component through the existing per-value
delimiter. Existing persistent dialog publication, persistent SNBT, registry
dependent shield defaults, effective-stack resolution, and native blocking
poses must provide named evidence before the ledger row becomes `complete`.

## Non-goals

- This slice does not encode `PIERCING_WEAPON`, `KINETIC_WEAPON`, or another
  remaining component, and does not mark the complete creative-slot packet
  surface supported.
- It does not add client-authoritative damage, durability, cooldown, or sound
  simulation. Those outcomes remain server authoritative; the observable
  native consumer in scope is effective component resolution and the held-item
  blocking pose.
- It does not reject non-finite or otherwise persistent-invalid floats at the
  network boundary. Vanilla's stream codec reads and writes raw floats; only
  the persistent SNBT/Codec projection applies `NON_NEGATIVE_FLOAT` and
  `POSITIVE_FLOAT`.
- It does not resolve dynamic damage-type or sound holder ids to resource ids
  while encoding the network form. Network holder ids are session ordinals and
  must retain their exact wire identity.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns exact network
  encoding, holder-set and sound-holder union validation, descriptor
  registration, and atomic output.
- `bbb-protocol::packets::data_components` owns bounded ordered decoding,
  bit-exact summaries, dynamic holder ids/tags, patch preflight, and delimited
  framing.
- `bbb-world` retains persistent dialog Codec decoding, registry-dependent
  shield defaults, and effective prototype-plus-patch resolution.
- `bbb-native` retains persistent SNBT range validation and consumes effective
  component presence for first- and third-person blocking poses.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.BLOCKS_ATTACKS` is persistent with `BlocksAttacks.CODEC`,
  network-synchronized with `BlocksAttacks.STREAM_CODEC`, and cache encoded.
- The network codec writes, in order: block-delay float, disable-cooldown-scale
  float, an ordered list of `DamageReduction`, one `ItemDamageFunction`, an
  optional damage-type holder set, an optional block sound holder, and an
  optional disable sound holder.
- Each reduction writes horizontal angle, optional damage-type holder set,
  base, and factor. The item-damage function writes threshold, base, and
  factor. All scalar fields use raw `ByteBufCodecs.FLOAT`.
- A holder set writes zero plus a tag identifier for a named set, or
  `size + 1` followed by exact session holder ids for a direct set. A sound
  holder writes `registry_id + 1`, or zero followed by a direct sound event
  resource id and optional fixed range.
- Persistent defaults are block delay `0`, cooldown scale `1`, one
  `90°/none/0/1` reduction, and item damage `1/0/1`. Persistent Codec ranges
  require non-negative block delay, cooldown scale, and threshold, plus a
  positive horizontal angle; base and factor are unrestricted `Codec.FLOAT`
  values, including the NBT representations of non-finite and negative-zero
  values.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps the complete value codec
  in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/BlocksAttacks.java`, `sounds/SoundEvent.java`,
`network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Add reusable exact network encoders for holder sets and sound holders, then
   register the type-37 encoder. Focused protocol tests cover both holder-set
   branches, registry and direct sounds, exact float bits, empty and maximum
   lists, invalid unions/identifiers/ids, and atomic rejection. Safe stop:
   protocol focused tests pass but no ledger claim exists.
2. Add patch-preflight and delimited/creative integration tests. Decode the
   encoded value through the same descriptor and compare the complete summary
   so stale or extra component state cannot escape. Safe stop: type 37 is the
   only newly encodable value.
3. Audit persistent dialog/SNBT, registry initialization, effective-stack, and
   blocking-pose evidence; close all six ledger cells, narrow the creative
   TODO, and archive this contract. Run affected focused tests, generator
   checks, and every per-commit check.

This is protocol/component high risk. The preceding written-book checkpoint
`58b874e1d` passed the full gate before implementation. Commit the complete
slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward before the
next slice.
