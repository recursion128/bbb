# Component-select codec parity engineering contract

## Goal and completion

Close Vanilla 26.1 `minecraft:component` item-model selection as one
codec-shaped vertical slice:

- every persistent built-in `DataComponentType` can decode `when` through its
  persistent codec and compare against the effective `ItemStack` value;
- object/list values, nested Components, float bit identity, holder/holder-set
  values, and recursive item stacks use semantic value equality rather than a
  lossy string or raw-JSON comparison;
- all source-derived item prototype component values are available to the same
  resolver, with explicit additions/removals taking precedence;
- item render consumers with a `ClientLevel`-equivalent world context use the
  current dynamic-registry generation when resolving holder-backed cases; and
- context-free consumers retain a deterministic built-in/static fallback and
  do not invent dynamic holder keys.

Completion requires reverse tests over all persistent component registrations,
all source-derived item prototype memberships, and every native item render
consumer family. The unsupported TODO is removed only after focused tests,
the full gate, and the applicable playable fixture pass at the final HEAD.

## Vanilla adjudications and non-goals

`ComponentContents.createType` dispatches the case codec from
`DataComponentType.codecOrThrow`, while rejecting `isTransient()` component
types before case decoding. `CREATIVE_SLOT_LOCK`, `ADDITIONAL_TRADE_COST`, and
`MAP_POST_PROCESSING` therefore cannot be legal component-select properties;
bbb will reject/fall back for them rather than synthesize a value codec.

`ComponentContents` resolves its component type through
`BuiltInRegistries.DATA_COMPONENT_TYPE`. Vanilla datapacks do not register new
data-component types or new item types. Supporting mod-loader-defined component
types/items is outside this native Vanilla 26.1 target. Resource-pack item
models may still use every persistent built-in component and datapack-created
stack values received through those built-in codecs.

This slice does not add an in-game pack/configuration UI, execute server
gameplay for component values, or introduce a second registry cache. It does
not compare serialized JSON text or flattened display text as a substitute for
Java value equality.

## Frozen ownership and integration boundaries

- `bbb-protocol` remains the only numeric component catalog and wire codec
  owner. It exposes a bounded one-value network encoding/equality surface from
  the existing descriptor table; component-bearing values use the existing
  canonical Component codec tree so shorthand and style-sensitive identity are
  both preserved.
- `bbb-world` remains the dynamic-registry and effective-prototype owner. Its
  existing persistent component decoder becomes the shared read-only bridge
  from one resource-pack `when` value to a typed component summary, and its
  registry revision remains the cache/generation identity.
- `bbb-pack::ItemRegistryCatalog` derives prototype membership from the
  checked-in Tree-sitter `Items.java` declaration IR and exact values from a
  checked-in report produced by Vanilla's own
  `DATA_COMPONENT_INITIALIZERS` plus each persistent codec. A reverse assertion
  requires every independently declared membership to have exactly one
  encodable value. No hand-maintained item membership or holder ordinal is
  added.
- `bbb-item-model` replaces the twelve-component enum with a descriptor-backed
  property (`type_id` plus resource id). Cases retain validated persistent
  values; selection asks the shared codec bridge for semantic equality against
  the effective patch. Existing scalar selects delegate to this path.
- `bbb-native` projects the world-owned effective stack and registry reference
  into GUI/HUD, dropped item, item frame, owner-backed held item, and
  entity-attached generated-item paths. It owns no decoded component value or
  registry-key copy.

Case declaration order and Vanilla's duplicate-value validation remain pack
load concerns. Runtime selection remains first matching case, then fallback.
Malformed/unknown/transient component properties never select an arbitrary
case.

## Authoritative Vanilla sources

- `client/renderer/item/properties/select/ComponentContents.java`
- `client/renderer/item/properties/select/SelectItemModelProperty.java`
- `client/renderer/item/SelectItemModel.java`
- `client/resources/model/ClientItemInfoLoader.java`
- `util/PlaceholderLookupProvider.java`
- `core/component/DataComponentType.java`
- `core/component/DataComponents.java`
- `world/item/Item.java` and `world/item/Items.java`

The equality contract follows decoded Java values. `SelectItemModel` stores
those values as map keys; when placeholder holders were used during resource
loading, `RegistryContextSwapper` codec-round-trips every key into the active
`ClientLevel.registryAccess()` before lookup.

## Serial stages, tests, and checkpoints

1. **Semantic codec bridge** (`bbb-protocol` + `bbb-world`): expose one
   persistent-value decode and one descriptor-backed semantic comparison path;
   add scalar, object/list, float, styled Component, holder, and transient
   rejection tests. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-protocol data_component` and
   `scripts/cargo-dev.sh test -p bbb-world persistent_component_select`.
2. **Generic select resolver** (`bbb-item-model`): replace the closed enum,
   preserve validated case values, and test explicit patches/removals across
   every persistent descriptor, including nested Component equality and
   registry remapping. Focused test:
   `scripts/cargo-dev.sh test -p bbb-item-model component_select`.
3. **Complete prototype projection** (`bbb-pack` + `bbb-world`): derive exact
   common, builder, delayed, and holder-backed item defaults; reverse-test all
   1,506 items so declared persistent memberships encode and patch precedence
   is exact. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-pack item_registry` and
   `scripts/cargo-dev.sh test -p bbb-world item_component_initializers`.
4. **Consumer integration and closeout** (`bbb-native` + ledgers): pass the
   effective stack and current registry owner through every level-backed item
   render family, retain context-free fallback, add source-structure and
   integration regressions, remove the TODO, archive this contract, and append
   the engineering milestone index. Focused test:
   `scripts/cargo-dev.sh test -p bbb-native component_select`.

Every stage ends in a buildable reviewed commit after the per-commit checks.
Because this changes codec and cross-crate ownership surfaces, run the full
gate after stage 2, after stage 4, at four accumulated commits, and immediately
after any abnormal focused-test failure. Stage boundaries are safe stopping
points: a failed later stage leaves the previous descriptor path buildable and
the remaining TODO plus this active contract explicit; history is fixed
forward and never rewritten.
