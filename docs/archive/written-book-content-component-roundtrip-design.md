# Written book content ItemStack component round-trip

## Goal and completion contract

Close all six ItemStack component coverage dimensions for Vanilla 26.1
`WRITTEN_BOOK_CONTENT` (type 55).

Completion means its catalog descriptor owns the exact
`WrittenBookContent.STREAM_CODEC` encoder: filterable title, author,
generation, ordered filterable Component pages, and resolved flag all
round-trip without loss. Page raw and filtered values retain their original
`ComponentSerialization.STREAM_CODEC` NBT; flattened page text is a derived
consumer projection and must match that value before encoding. Title/author
network strings use Java UTF-16 limits, generation remains in 0 through 3,
list and NBT budgets stay bounded, and inconsistent aligned vectors,
malformed Components, stale projections, or missing values fail before patch
bytes are appended.

Creative ItemStacks must encode the component through the existing per-value
delimiter. Existing persistent dialog publication, persistent SNBT, tooltip,
book-page, and item-model predicate consumers must provide named evidence
before the ledger row becomes `complete`.

## Non-goals

- This slice does not encode `WRITABLE_BOOK_CONTENT` or any other remaining
  component type, and does not mark `SERVERBOUND_SET_CREATIVE_MODE_SLOT`
  complete.
- It does not implement a native book editing or reading screen, alter page
  command/dialog resolution, or mutate the server-authored `resolved` bit.
- It does not reconstruct Component NBT from flattened page text. The original
  validated NBT is the only reverse-codec source.
- It does not apply the persistent `flatRestrictedCodec(32767)` JsonOps page
  size rule to the network StreamCodec, which uses the ordinary Component
  network codec. Persistent dialog decoding keeps its existing canonical JSON
  limit owner.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns exact network
  encoding, aligned-vector/canonical validation, descriptor registration, and
  atomic output.
- `bbb-protocol::packets::data_components` owns bounded ordered decoding,
  Java UTF-16 string checks, generation validation, original page Component
  NBT retention, derived text, patch preflight, and delimited framing.
- `bbb-world` retains persistent dialog Codec decoding and publishes raw plus
  filtered Component trees into the canonical patch summary.
- `bbb-native` retains persistent SNBT projection and persistent Codec range
  checks. `bbb-item-model` retains book tooltip and exact/predicate consumers.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.WRITTEN_BOOK_CONTENT` is persistent with
  `WrittenBookContent.CODEC`, network-synchronized with
  `WrittenBookContent.STREAM_CODEC`, and cache encoded.
- The network codec writes, in order:
  `Filterable.streamCodec(stringUtf8(32))` title, `STRING_UTF8` author,
  generation VarInt, an ordered unbounded-codec list of
  `Filterable.streamCodec(ComponentSerialization.STREAM_CODEC)` pages, and a
  resolved boolean.
- `Filterable.streamCodec` writes the raw value followed by an optional
  filtered value as presence boolean plus payload.
- The record constructor accepts generation only in 0 through 3. Network title
  length is at most 32 Java UTF-16 units; author uses the common 32767-unit
  string bound.
- The persistent Codec uses the same filterable shapes, defaults generation,
  pages, and resolved, and applies the 32767-UTF-16-unit canonical JsonOps
  restriction to each page.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps the complete value codec
  in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/WrittenBookContent.java`,
`server/network/Filterable.java`,
`network/chat/ComponentSerialization.java`,
`network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Tighten written-book network title/author decoding to Java UTF-16 limits and
   enforce the generation constructor domain. Focused protocol tests cover
   exact astral boundaries, out-of-range generation, both filterable branches,
   and exact raw/filtered Component NBT. Safe stop: no encode callback or
   ledger claim yet.
2. Add the descriptor encoder and canonical patch preflight. Focused tests
   prove exact field order, aligned optional page values, Component
   canonicality, list limits, atomic rejection, per-value framing, and
   creative packet integration. Safe stop: type 55 remains the only newly
   encodable value.
3. Correct the persistent title UTF-16 check, audit existing
   dialog/tooltip/book/predicate evidence, close all six ledger cells, narrow
   the creative TODO, and archive this contract. Run affected-crate focused
   tests, generator checks, and every per-commit check.

This is protocol/component high risk. The preceding attribute-modifier
checkpoint `4bafba912` passed the full gate before implementation. Commit the
complete slice, then run `scripts/cargo-dev.sh gate`; fix any failure forward
before the next slice.
