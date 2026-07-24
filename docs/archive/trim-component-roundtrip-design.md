# Armor trim ItemStack component round-trip

## Goal and completion contract

Close all six ItemStack coverage dimensions for Vanilla 26.1 `TRIM`
(type 56) as one protocol-to-render engineering slice.

Completion means the catalog descriptor owns an exact network encoder for
both `ArmorTrim` holders. Registry references retain the decoded zero-based
dynamic-registry holder id and encode it as `id + 1`; direct holders use the
zero sentinel and preserve every direct material or pattern field. Direct
component descriptions must retain their complete canonical Component NBT,
styled projection, and flattened text. Malformed holder unions, invalid
resource paths or identifiers, oversized override maps, missing component
NBT, and stale projections fail before any patch bytes are appended.

Creative ItemStacks must preserve independent per-value delimiters. Existing
persistent dialog/SNBT, recipe preview, exact item-model predicate, native
tooltip, dynamic registry, and named/direct armor-render consumers provide
the semantic evidence required before the ledger row becomes `complete`.

## Non-goals

- This slice does not encode `DEBUG_STICK_STATE`, `PROVIDES_TRIM_MATERIAL`, or
  another ItemStack component.
- It does not change trim registry synchronization, texture loading, sprite
  construction, recipe logic, or item-model predicate semantics.
- It does not impose persistent Codec registry membership on dynamic network
  holder ids. Network references are valid for any non-negative id whose
  `id + 1` fits a signed VarInt.
- It does not preserve source map insertion order. The decoded summary owns a
  semantic `BTreeMap`; the encoder writes its deterministic key order.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns the exact type-56
  network encoder, Holder union validation, direct payload validation,
  descriptor registration, and atomic output.
- `bbb-protocol::packets::data_components` retains typed decoding, canonical
  summaries, patch preflight, and delimited framing.
- `bbb-world` retains dynamic trim registry identity, recipe preview,
  persistent dialog projection, and named/direct armor sprite derivation.
- `bbb-item-model` retains trim predicates and tooltip presentation;
  `bbb-native` retains persistent SNBT and entity equipment rendering.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.TRIM` is persistent, network-synchronized with
  `ArmorTrim.STREAM_CODEC`, and caches its encoding.
- `ArmorTrim.STREAM_CODEC` writes the material Holder followed by the pattern
  Holder.
- `ByteBufCodecs.holder` writes a registry reference as `registry id + 1`;
  zero denotes a direct holder followed by its direct codec.
- `TrimMaterial.DIRECT_STREAM_CODEC` writes
  `MaterialAssetGroup.STREAM_CODEC` followed by
  `ComponentSerialization.STREAM_CODEC`. The asset group writes a resource
  path string, a map of equipment-asset identifiers to resource path strings,
  then the component.
- `TrimPattern.DIRECT_STREAM_CODEC` writes an asset identifier, component,
  then boolean `decal`.
- `ArmorTrim.layerAssetId` selects the material override for the equipped
  armor asset and composes the pattern and suffix into the trim sprite id;
  `ArmorTrim.addToTooltip` uses both direct descriptions.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` length-prefixes each complete
  component value independently.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`core/component/DataComponentPatch.java`,
`network/codec/ByteBufCodecs.java`, and
`world/item/equipment/trim/{ArmorTrim,TrimMaterial,TrimPattern,MaterialAssetGroup}.java`.

## Serial stages and validation

1. Add the exact descriptor encoder and focused protocol tests covering
   registry/direct/mixed holders, override maps, styled Component NBT,
   dynamic ids, malformed unions, invalid paths/ids, missing/stale component
   projections, count bounds, and atomic failure. Safe stop: descriptor tests
   pass with no ledger claim.
2. Add canonical patch preflight, per-value delimiter, and exact creative
   packet tests. Re-run persistent SNBT/dialog, item-model predicate/tooltip,
   recipe preview, dynamic registry, and named/direct entity render focused
   tests. Safe stop: type 56 is the only newly encodable value.
3. Close the six ledger cells, narrow the creative TODO, archive this
   contract, and add the engineering index entry. Run affected focused tests,
   generator checks, and every per-commit check.

This is protocol/component high risk. The preceding weapon-interaction
checkpoint `d3d891112` passed the full gate before implementation. Commit the
complete slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward
before the next slice.
