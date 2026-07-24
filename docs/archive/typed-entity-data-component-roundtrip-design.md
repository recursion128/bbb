# Typed entity-data ItemStack component round-trip

## Goal and completion contract

Close all six item-component coverage dimensions for the related Vanilla 26.1
components:

- `ENTITY_DATA`;
- `BUCKET_ENTITY_DATA`; and
- `BLOCK_ENTITY_DATA`.

Completion means each descriptor owns an exact network encoder;
`TypedEntityData` values validate their fixed entity/block-entity registry
holder, require an unnamed non-null `CompoundTag`, and remove any embedded
`id` field exactly as the Vanilla constructor does; bucket entity data requires
and preserves its compound unchanged. Creative ItemStacks must encode all
three through the existing per-value delimiter, while missing, scalar,
out-of-range-holder, dangling-derived, and otherwise noncanonical summaries
fail before patch bytes are appended.

The existing canonical dialog/publication paths, persistent SNBT projection,
entity-data peaceful warning, spawner description, operator warning, and
component tests must provide named evidence before any ledger cell becomes
`complete`.

## Non-goals

- This slice does not encode component types 5 through 57 or 61 through 109.
- It does not mark `SERVERBOUND_SET_CREATIVE_MODE_SLOT` complete.
- It does not add custom/datapack entity or block-entity registry types; both
  Vanilla registries are fixed built-in protocol registries.
- It does not implement entity/block-entity mutation prediction from these
  ItemStack values; the server remains authoritative.
- It does not close the structurally related `BEES` component, although its
  nested `TypedEntityData<EntityType<?>>` decoder must obey the same holder,
  compound-root, and `id`-stripping contract.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns fixed-registry
  validation, typed-data `id` stripping, required-compound validation, and the
  three descriptor encode callbacks.
- `bbb-protocol::packets::data_components` owns summary fields, derived
  `block_entity_spawn_entity_type`, canonical patch preflight, and delimited
  framing.
- `bbb-protocol::nbt_summary` remains the sole binary NBT encoder/decoder.
- `bbb-native` retains persistent SNBT ownership: typed data re-inserts the
  resource `id` for the persistent Codec, while bucket data remains the exact
  compound.
- `bbb-world` retains dialog component decoding/application and canonical
  state. `bbb-item-model` retains peaceful/spawner/operator tooltip consumers.
- Sparse coverage overrides remain the sole hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.java` registers `ENTITY_DATA` and `BLOCK_ENTITY_DATA` with
  `TypedEntityData.streamCodec(...)`; `BUCKET_ENTITY_DATA` uses
  `CustomData.STREAM_CODEC`.
- `TypedEntityData.streamCodec` writes the registry value first, then
  `ByteBufCodecs.COMPOUND_TAG`.
- Its constructor calls `stripId`: an embedded top-level `id` is removed from
  network data. Its persistent Codec performs the inverse, adding the typed
  resource identity as the compound's `id` field.
- `CustomData.STREAM_CODEC` is `ByteBufCodecs.COMPOUND_TAG`; null/end and scalar
  roots are invalid.
- `TypedEntityData.addToTooltip` shows the peaceful warning for disallowed
  entity types. Existing spawner and operator-only consumers follow the same
  typed holder and retained tag.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps each of these complete
  value codecs in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/TypedEntityData.java`,
`world/item/component/CustomData.java`,
`network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Add registry-aware typed-data decoding, required-compound validation, and
   top-level `id` stripping. Focused protocol tests cover exact entity,
   block-entity, bucket, and nested-BEES decode plus null/scalar/unknown-holder
   failures. Safe stop: no encoder or ledger claim until all receive paths pass.
2. Add descriptor callbacks for holder-plus-compound and compound-only values,
   then extend canonical preflight. Focused tests prove exact descriptor bytes,
   added/removal order, derived spawner identity, missing/stale-field
   rejection, and creative packet integration. Safe stop: all encoding remains
   fail-closed until the three callbacks are fully preflighted.
3. Tighten bucket persistent projection, audit existing world/item-model
   consumers, close the three six-dimensional ledger rows, narrow the remaining
   creative TODO, and archive the contract. Run affected-crate focused tests,
   generator checks, and every per-commit check.

This is protocol/NBT high risk: commit the complete slice, then run
`scripts/cargo-dev.sh gate`. Any failure is fixed forward before the next
slice.
