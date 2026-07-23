# Update Recipes Packet Closure Engineering Contract

This document freezes the high-risk engineering slice that closes
`CLIENTBOUND_UPDATE_RECIPES` for Vanilla 26.1. The packet wire shell, property
sets, stonecutter input holder sets, replacement lifecycle, menu filtering,
scrolling, and direct item/item-stack buttons already exist. The remaining gap
is the exact semantic decode and `resolveForFirstStack` presentation path for
all 11 built-in `SlotDisplay` types.

The latest clean baseline is commit `44a1dfd4e`. Its full gate passed all six
steps, and its playable auto gate passed all eight fixture phases. The explicit
live phase was unavailable because no Java executable was configured.

## Goal and Completion

An ordered packet must replace one immutable client recipe-access generation:

```text
CLIENTBOUND_UPDATE_RECIPES
  -> bounded property-set and selectable-recipe decode
  -> typed, bounded SlotDisplay tree
  -> atomic ClientRecipesState replacement
  -> current world SlotDisplayContext resolution
  -> Stonecutter item/tooltip presentation
```

The slice is complete when every built-in `SlotDisplays.bootstrap` type has an
exact typed wire representation and a canonical stack resolver; recursive
payloads have an explicit byte/depth/count budget; item, tag, fuel, potion,
component, dye, and trim inputs resolve from the current item prototypes and
frozen registry/tag revision; malformed or unresolved authoritative holders do
not partially replace the previous recipe generation; Stonecutter buttons and
tooltips consume the shared resolver without reparsing bytes in `bbb-native`;
the packet's five coverage cells are complete; the remaining recipe-book and
ghost-recipe presentation TODO is narrowed to those separate packets; and the
final full and playable gates pass.

## Non-Goals

- Do not broaden this packet into `RecipeBookAdd`, `PlaceGhostRecipe`, recipe
  toast, narration, automatic `RecipeBookSeenRecipe`, or crafting-result
  prediction. They may reuse the resolver later but retain their own packet/UI
  ledger gaps.
- Do not implement server recipe assembly or infer a Stonecutter result. The
  server remains authoritative for result slots; this packet owns selectable
  input filtering and option presentation only.
- Do not add a second item registry, tag map, fuel table, potion order, trim
  order, or native `SlotDisplay` id switch.
- Do not resolve a synchronized holder by a fixed bootstrap ordinal when the
  current session registry is authoritative.
- Do not use wall-clock, address identity, or thread-local randomness. Vanilla's
  `SmithingTrimDemoSlotDisplay` seeds a display-only random sample from object
  identity; bbb uses a stable payload-derived seed while preserving the same
  candidate/filter/limit semantics.
- Do not add in-game configuration UI.

## Owners and Frozen Integration Boundary

- `bbb-protocol` owns the 11 generated type ids, recursive stream decode, exact
  item-stack template/component patch, named item tag, component type id, and
  named/direct trim-pattern holder. `SlotDisplaySummary` keeps its exact raw
  payload for compatibility and exposes one typed decode API; consumers do not
  implement a second parser.
- `bbb-pack` continues to derive item prototype facts from local Vanilla source
  at maintenance time. It exposes exact default `DYE` values and
  `PROVIDES_TRIM_MATERIAL` initializer keys by item protocol id; ordinary
  build/test reads checked-in generated/catalog state only.
- The gameplay-id generator owns `DyeColor` ids and texture diffuse colors.
  World and renderer consumers must not introduce a second color table.
- `bbb-world::WorldStore` owns the canonical `SlotDisplayContext`: item tags,
  furnace fuel items, built-in potion order, dynamic trim holders, and effective
  item prototype plus explicit patch semantics. Its resolver returns ordered
  `ItemStackSummary` candidates and is the only place that applies component,
  dye, potion, and trim transformations.
- `bbb-native` owns only Stonecutter layout, selection, tooltip, and item draw
  submission. It asks `WorldStore` for the first resolved stack and removes its
  local raw-payload decoder.

The public integration surface is limited to a typed protocol value/error API
and `WorldStore` stack-resolution queries. No mutable registry or item-runtime
owner crosses crate boundaries.

## Vanilla 26.1 Evidence

Local source root: `/home/zgy/Work/mc-code/sources/26.1`.

- `ClientboundUpdateRecipesPacket.STREAM_CODEC` contains a map of
  `RecipePropertySet.STREAM_CODEC` followed by
  `SelectableRecipe.SingleInputSet.noRecipeCodec()`.
- `ClientPacketListener.handleUpdateRecipes` replaces `ClientRecipeContainer`
  wholesale. `ClientRecipeContainer` returns an empty property set for a missing
  key and retains the exact Stonecutter selectable list.
- `SelectableRecipe.SingleInputEntry.noRecipeCodec` carries
  `Ingredient.CONTENTS_STREAM_CODEC` followed by one `SlotDisplay`; it carries
  no server recipe/result payload.
- `SlotDisplays.bootstrap` registers, in order: `empty`, `any_fuel`,
  `with_any_potion`, `only_with_component`, `item`, `item_stack`, `tag`,
  `dyed`, `smithing_trim`, `with_remainder`, and `composite`.
- `SlotDisplayContext.fromLevel` supplies exactly `FuelValues` and the current
  `RegistryAccess`.
- `Empty` resolves nothing. `AnyFuel` resolves current fuel items. `Item`,
  `ItemStack`, and `Tag` resolve their fixed holder, exact template, and current
  item tag respectively. `Composite` concatenates children in order.
- `WithRemainder` resolves only its input for the stack factory.
  `OnlyWithComponent` filters source stacks by their effective component.
  `WithAnyPotion` clones every source stack for every current potion holder and
  sets `POTION_CONTENTS`.
- `DyedSlotDemo` combines target and dye candidates in target-major Cartesian
  order, applies `DyedItemColor.applyDyes` with the dye stack's `DYE` component
  or white fallback, removes empty results, and limits output to 16.
- `SmithingTrimDemoSlotDisplay` samples base/material pairs, applies the
  material stack's `PROVIDES_TRIM_MATERIAL` plus the named/direct pattern,
  drops unchanged/invalid results, and limits output to 16.
- `StonecutterMenu` filters the synchronized list by the current input
  ingredient. `StonecutterScreen` calls `resolveForFirstStack` for both the
  button item and hover tooltip; the server owns selected result contents.

The main agent rechecks the complete recursive codecs, `DyedItemColor` mixing,
`SmithingTrimRecipe.applyTrim`, item prototype initializers, and dynamic holder
semantics before the corresponding implementation stage.

## Typed Decode and Resolution Contract

The typed value is closed over the 11 generated ids. Nested displays decrement a
single depth budget; every list is count-bounded and every tree is charged to
the packet byte budget before allocation. Unknown ids, negative/dangling
holders, invalid component ids, invalid item templates, over-depth trees, and
trailing nested bytes fail explicitly. A direct trim pattern preserves asset
id, exact styled Component/NBT, and decal flag.

Resolution always starts from the typed value and the current immutable world
revision:

- explicit item-stack patches override item prototype defaults; removals beat
  additions;
- missing item tags, unavailable context, and transformations with no valid
  candidates resolve to an empty list rather than inventing a fallback;
- resolved candidate order follows source display order, holder/tag order, and
  Vanilla transform order;
- every transformed stack has count one where Vanilla uses `copyWithCount(1)`,
  retains unrelated components, replaces the target component atomically, and
  updates component presence/removal bookkeeping;
- successful `resolveForFirstStack` takes the first candidate; no presentation
  path decodes raw bytes itself.

Packet application validates every authoritative item/trim holder needed to
interpret the retained tree before swapping generations. Named display tags may
legitimately be absent and resolve empty, matching the optional registry tag
lookup. A rejected packet increments receipt diagnostics but leaves the prior
property sets, recipes, and derived counters unchanged.

## Serial Stages, Tests, and Gates

1. **Typed protocol milestone.** Add the closed typed value/parser, named/direct
   trim pattern retention, recursive budget, and fixtures for every built-in
   branch, boundary depth, malformed holder/type/component, and exact raw
   round-trip. Run focused `bbb-protocol` recipe tests, the packet-catalog
   generator check, formatting, diff check, and complete review. Commit and run
   the full gate because packet decode is high risk. This gated commit is the
   first safe stop.
2. **Canonical context and resolver.** Extend generated dye-color facts and
   source-derived item prototype values; project them through resource reload
   into `WorldStore`; implement all stack transformations and atomic packet
   validation. Test custom/reordered trim holders, tag replacement/removal,
   fuel refresh, all potions, component add/remove/default precedence, exact dye
   mixing, deterministic trim sampling, empty contexts, reset, serialization,
   and publication isolation. Run affected generator/pack/world focused tests,
   both relevant generator checks, per-stage checks, commit, and run the full
   gate because this stage changes a generator.
3. **Native presentation and packet closure.** Replace the native
   item/item-stack raw decoder with the shared first-stack resolver; test all 11
   display shapes through UpdateRecipes→world→Stonecutter button and tooltip,
   including current-session tag/trim changes. Close the packet ledger, narrow
   the separate recipe-book/ghost TODO, archive this contract, and review the
   integrated diff. Run protocol coverage plus affected world/native tests and
   all per-stage checks. Commit, run the full gate, then playable live
   explicitly and playable auto according to repository policy.

A stage failure is fixed forward within the same stage. Protocol failures are
localized by branch/depth fixtures; source/default failures by generator and
resource-reload tests; resolver failures by one-variant-at-a-time world tests;
native failures by deterministic Stonecutter extraction fixtures. No later stage
starts until the prior commit is clean. If a required prototype value cannot be
derived exactly or a dynamic holder cannot use the frozen registry revision, the
safe stop is the latest fully gated milestone with the packet row still partial.
