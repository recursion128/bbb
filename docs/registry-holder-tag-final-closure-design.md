# Registry holder and tag final coverage closure

## Goal and completion contract

Close every remaining partial row in the vanilla 26.1 registry-holder consumer
ledger. The one clientbound wire consumer,
`SlotDisplay.TagSlotDisplay.STREAM_CODEC`, must retain a normalized item tag and
resolve its ordered members from the current `UpdateTags` revision at every
recipe-book and ghost-display read. Tag replacement and removal must become
visible without a parallel cache. Every remaining persistent/generic row must
receive a source-backed server-only or helper-boundary decision.

Completion means the 194-row ledger contains no `partial` status, the generated
ledger and override source agree, all status/count guards pass, and the
workspace full gate plus applicable playable fixture smoke pass.

## Scope and non-goals

- `bbb-protocol` remains the owner of SlotDisplay stream decoding and exact raw
  nested payload retention.
- `bbb-world` remains the owner of `UpdateTags` replacement semantics and
  canonical recipe state.
- `bbb-native` remains the only recipe presentation/craftability consumer and
  reads the world-owned item-tag map on demand.
- Coverage-family overrides own the client/server classification; the
  generated ledger is never hand-edited.

This slice does not add recipe JSON loading, an integrated server, block codec
loading, advancement predicate execution, loot execution, or world generation.
It does not reinterpret persistent codecs as packet codecs, and it does not
introduce a second tag cache or bootstrap ordinal fallback.

## Vanilla evidence and classification contract

- `TagSlotDisplay.STREAM_CODEC` uses
  `TagKey.streamCodec(Registries.ITEM)` and is nested by
  `RecipeDisplay.STREAM_CODEC` in RecipeBookAdd, PlaceGhostRecipe, and
  UpdateRecipes display payloads. `TagSlotDisplay.resolve` queries the supplied
  item registry lookup when the display is consumed.
- `TagSlotDisplay.MAP_CODEC` and `Ingredient.CODEC` are persistent recipe data
  codecs. Clientbound recipe contents use SlotDisplay and Ingredient stream
  codecs instead.
- `Biome.LIST_CODEC` uses `Biome.DIRECT_CODEC`, while
  `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` explicitly selects
  `Biome.NETWORK_CODEC`; configured carvers/features/placements are server
  world-generation registries.
- TagPredicate is an advancement-criterion predicate. TagEntry and
  ExplorationMapFunction execute against server loot context, and the latter
  requires `ServerLevel`.
- Block MapCodecs retain constructor/bootstrap data; growth, bonemeal, and
  placement mutations that use their TagKeys are server authority. bbb builds
  immutable client block identity/state catalogs from vanilla reports instead
  of loading block MapCodecs.
- Geode, root-system, vegetation-patch, protected-block, matching-block-tag,
  and tag-match codecs are world-generation/structure processing.
- Generic `RegistryCodecs` and `HolderSetCodec` calls delegate shape creation;
  their concrete consumers are inventoried separately and must not count as a
  second client behavior.

Authoritative sources are the corresponding files under
`~/Work/mc-code/sources/26.1/net/minecraft/`, especially `SlotDisplay.java`,
`Ingredient.java`, `Biome.java`, `RegistryDataLoader.java`, and the owning
block/worldgen/advancement/loot classes named by the ledger rows.

## Serial stages and verification

1. Add a native semantic regression proving a retained SlotDisplay tag follows
   reordered and removed item-tag revisions while the protocol tag decoder
   retains normalized identity. Focused tests: the named `bbb-protocol` decoder
   test and named `bbb-native` recipe UI test, plus non-test checks for both
   crates.
2. Split the five remaining broad coverage families into the one complete
   network-tag consumer and server-only/helper families, regenerate the ledger,
   remove the finished unsupported-feature item, and update count/reverse
   guards. Focused tests: registry-holder coverage tests, generator unit tests,
   generator `--check`, docs lint, and non-test protocol check.
3. Append the milestone index entry, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `087266932` is the pre-start checkpoint. Each normal stage is a
clean, independently buildable commit. A classification mismatch stops in the
override/generator stage; a presentation mismatch stops in the focused native
test. Failures are fixed forward without rewriting history or publishing a
partially regenerated ledger.
