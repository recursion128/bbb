# Attribute modifiers ItemStack component round-trip

## Goal and completion contract

Close all six ItemStack component coverage dimensions for Vanilla 26.1
`ATTRIBUTE_MODIFIERS` (type 16).

Completion means its catalog descriptor owns the exact ordered
`ItemAttributeModifiers.STREAM_CODEC` encoder: the fixed attribute holder,
modifier identifier, raw double bits, operation, equipment-slot group, and
display union all round-trip without loss. Override displays retain the
original `ComponentSerialization.STREAM_CODEC` NBT as the canonical value;
flattened text and styled runs remain derived consumer projections and must
match that value before encoding. Invalid fixed holders, noncanonical
identifiers, invalid enum ids, incoherent display fields, oversized lists, and
malformed Components fail before patch bytes are appended.

Creative ItemStacks must encode the component through the existing per-value
delimiter. Existing persistent dialog publication, persistent SNBT, tooltip,
and item-model predicate consumers must provide named evidence before the
ledger row becomes `complete`.

## Non-goals

- This slice does not encode component types 5 through 15, 17 through 57, 61
  through 69, or 71 through 109, and does not mark
  `SERVERBOUND_SET_CREATIVE_MODE_SLOT` complete.
- It does not change dynamic attribute-registry ownership. The network
  component uses the built-in `Attribute.STREAM_CODEC` catalog already owned
  by `bbb-protocol`; persistent dialog decoding continues to resolve resource
  ids through its existing registry context.
- It does not change attribute computation, equipment activation, default item
  tables, tooltip formatting, or item-model predicate matching.
- It does not reconstruct Component NBT from flattened text or styled runs.
  The original validated NBT is the only reverse-codec source.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns exact network
  encoding, enum/holder/identifier validation, descriptor registration, and
  atomic output.
- `bbb-protocol::packets::data_components` owns bounded ordered decoding,
  original override Component NBT retention, derived text/runs, patch
  preflight, and delimited framing.
- `bbb-world` retains persistent dialog Codec decoding and publishes the
  dialog Component value into the canonical patch summary.
- `bbb-native` retains persistent SNBT projection, including the override
  display record. `bbb-item-model` continues to own tooltips, effective
  defaults, and exact/predicate consumers.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.ATTRIBUTE_MODIFIERS` is persistent with
  `ItemAttributeModifiers.CODEC`, network-synchronized with
  `ItemAttributeModifiers.STREAM_CODEC`, and cache encoded.
- The network codec writes an ordered list. Each `Entry.STREAM_CODEC` writes
  `Attribute.STREAM_CODEC`, `AttributeModifier.STREAM_CODEC`,
  `EquipmentSlotGroup.STREAM_CODEC`, and `Display.STREAM_CODEC`, in that order.
- `AttributeModifier.STREAM_CODEC` writes an `Identifier`, double, and
  operation id. Operations are ids 0 `add_value`, 1
  `add_multiplied_base`, and 2 `add_multiplied_total`; out-of-range network
  ids decode with Vanilla's zero fallback.
- Equipment-slot groups use their generated ids and the same zero-fallback
  behavior.
- Display is a dispatched union: id 0 default and id 1 hidden have no payload;
  id 2 override writes one required `ComponentSerialization.STREAM_CODEC`
  value. The persistent Codec represents hidden/override as typed records and
  omits the default display field.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps the complete value codec
  in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/ItemAttributeModifiers.java`,
`world/entity/ai/attributes/AttributeModifier.java`,
`world/entity/EquipmentSlotGroup.java`,
`network/chat/ComponentSerialization.java`,
`network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Extend the canonical modifier summary with original override Component NBT
   and thread it through protocol decode plus persistent dialog publication.
   Focused protocol/world tests cover default, hidden, override, exact NBT,
   flattened projections, and invalid display shapes. Safe stop: no encode
   callback or ledger claim yet.
2. Add the descriptor encoder and canonical patch preflight. Focused tests
   prove exact field order and raw double bits, all display variants,
   fixed-holder/identifier/enum validation, atomic rejection, per-value
   framing, and creative packet integration. Safe stop: type 16 remains the
   only newly encodable value.
3. Complete override persistent SNBT projection, audit existing
   tooltip/item-model evidence, close all six ledger cells, narrow the
   creative TODO, and archive this contract. Run affected-crate focused tests,
   generator checks, and every per-commit check.

This is protocol/component high risk. The preceding profile checkpoint
`41b22c8dd` passed the full gate before implementation. Commit the complete
slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward before
the next slice.
