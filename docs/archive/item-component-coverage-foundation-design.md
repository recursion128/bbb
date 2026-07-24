# Item-component semantic coverage foundation

## Goal and completion contract

Create one generated, reviewable inventory for every Vanilla 26.1
`DataComponentType` that can appear in an `ItemStack` patch. The inventory must
separate these independent claims:

- exact typed network decode;
- lossless canonical retention and equality;
- network re-encoding;
- persistent projection when Vanilla defines a persistent Codec;
- client-observable consumers; and
- semantic tests.

Completion of this foundation means all 110 entries are derived in Vanilla
registration order, their value type and Codec-selection facts are recorded,
sparse status overrides are validated against real evidence, and repository
tests prevent component-dependent packet cells from being closed while this
ledger still contains incomplete semantics. A decoder consuming the expected
number of bytes or retaining only the component type id is not completion.

## Non-goals

- This foundation does not change an ItemStack wire decoder, canonical value,
  encoder, renderer, tooltip, inventory prediction, or packet coverage status.
- It does not pre-classify existing summary fields as lossless. Each completed
  cell needs a later source-backed audit and focused semantic test.
- It does not copy Mojang Codec implementations wholesale or introduce a
  second data-component catalog.

## Ownership and integration boundary

- `scripts/core_registry_catalog_extractor.py` owns Tree-sitter-derived
  `DataComponents.java` facts: registration order, resource id, value type,
  persistent Codec, explicit network codec, persistent-codec fallback,
  `cacheEncoding`, and `ignoreSwapAnimation`.
- `scripts/generate-core-registry-catalog.py` owns the checked-in JSON ledger
  and merges a sparse evidence override file. New Vanilla entries default every
  semantic cell to `partial`.
- `bbb-protocol::item_component_coverage` owns schema parsing and reverse
  assertions against the existing generated component catalog and packet
  coverage ledger. It does not become a runtime ItemStack owner.
- The existing data-component catalog remains the only source of numeric type
  ids in production Rust.

## Vanilla 26.1 evidence

- `DataComponents.java` defines the 110 registrations and their builder
  properties.
- `DataComponentType.Builder.build` selects an explicit
  `networkSynchronized` codec or falls back to
  `ByteBufCodecs.fromCodecWithRegistries(persistentCodec)`.
- `DataComponentPatch.STREAM_CODEC` writes positive entries as component type
  plus the selected value codec and negative entries as component type only.
- `ItemStack.OPTIONAL_STREAM_CODEC` carries that patch after the item holder.

The three transient registrations (`CREATIVE_SLOT_LOCK`,
`ADDITIONAL_TRADE_COST`, and `MAP_POST_PROCESSING`) have network codecs but no
persistent projection; only that one cell is generated as `not-applicable`.

## Serial stages and validation

1. Extend the shared Java extractor with Codec-selection facts and focused
   extractor tests.
2. Generate the full JSON inventory, validate sparse overrides and evidence,
   and add generator unit tests.
3. Add Rust schema/reverse tests, ledger routing documentation, and the
   component-dependent packet closure guard.

Focused validation is the two affected Python test modules plus
`bbb-protocol` item-component coverage tests. Because the slice changes a
generator and protocol closure gate, its single integration commit requires
the full repository gate. A failing extractor or generated-file comparison
localizes failure before any runtime code is changed; the checked-in prior
catalog remains buildable at every stopping point.
