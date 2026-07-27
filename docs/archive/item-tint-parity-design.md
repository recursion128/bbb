# Dynamic Item Tint Parity

Status: completed on 2026-07-26.

This document freezes the engineering contract for closing the remaining
resource-pack item-tint gap. It refines the item ownership boundaries in
[`native-client-architecture.md`](native-client-architecture.md), the selected
resource-generation contract in
[`resource-pack-lifecycle-design.md`](resource-pack-lifecycle-design.md), and
the resolved-leaf contract in
[`special-item-model-rendering-design.md`](special-item-model-rendering-design.md).

## Goal and Completion

All eight Java 26.1 `ItemTintSources` variants must produce the same opaque
colour as Vanilla for every implemented item rendering path:

- `constant` and `grass` remain resource-static;
- `custom_model_data`, `dye`, `map_color`, and `firework` continue reading the
  effective stack component patch;
- `potion` uses `PotionContents.getColorOr`: explicit custom colour first,
  otherwise the amplifier-weighted colour of every visible base-potion and
  custom effect, otherwise the source default; and
- `team` uses the living owner's scoreboard-team formatting colour, otherwise
  the source default.

Completion requires one tint evaluation in `bbb-item-model` to feed ordinary
generated layers, cuboid leaves, composite leaves, bundle-selected leaves, GUI
icons, world items, held/head items, and first-person items. Native callers
must distinguish Vanilla's living-owner paths from fake or non-living paths.
Reload keeps using the accepted immutable item-model generation; no renderer
handle or scoreboard state enters the pack or item-runtime owner.

## Non-Goals

- No packet format, component schema, resource selection, atlas upload, mip,
  animation, foil, lighting, display transform, model selection, or draw-order
  change.
- No new scoreboard owner or renderer-side team lookup.
- No dynamic-registry replacement for potion or mob-effect ids. Vanilla 26.1
  keeps both registries client built-in, and the existing generated catalogs
  remain authoritative.
- No expansion of deferred entity-in-UI item-layer drawing or unimplemented
  block-entity item presentation.

## Ownership and Integration Contract

- `bbb-protocol` continues to own the generated potion effects, generated
  mob-effect colours, and exact `DataComponentPatchSummary`.
- `bbb-world` owns scoreboard membership and exposes only a narrow optional RGB
  query for an entity or the local player. Missing entity, team, parameters,
  reset formatting, or non-colour formatting returns `None`.
- `bbb-item-model` owns all eight tint calculations. Its resolver accepts one
  optional living-owner team RGB value alongside the existing owner context;
  recursive composite and bundle resolution inherits that value unchanged.
- `bbb-native` decides whether Vanilla supplied a living owner. Default
  `GuiGraphicsExtractor.item`, first-person, held, and head paths pass the
  matching living entity's team colour. `fakeItem`, dropped items, item frames,
  thrown projectiles, fireworks, item displays, and other non-living owners pass
  no team colour.
- `bbb-renderer` continues to consume the final per-layer RGBA value. It gains
  no scoreboard or component dependency.

Fallback is exact and local: a missing component, effect catalog entry, living
owner, team, or formatting RGB uses the tint source's declared default. Unknown
or malformed values remain rejected by their existing protocol/pack boundary.

## Vanilla 26.1 Evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `net/minecraft/client/color/item/ItemTintSources.java` registers exactly the
  eight supported source types.
- Each class in `net/minecraft/client/color/item/` defines its component/default
  rule. `TeamColor.calculate` reads only
  `owner.getTeam().getColor().getColor()`.
- `net/minecraft/world/item/alchemy/PotionContents.java#getColorOr` gives custom
  colour precedence, concatenates base and custom effects, filters by
  `MobEffectInstance.isVisible`, and weights RGB by `amplifier + 1`.
- `net/minecraft/client/renderer/item/CuboidItemModelWrapper.java#update` calls
  every source with `owner.asLivingEntity()`.
- `ItemModelResolver`, `GuiGraphicsExtractor`, entity render-state extractors,
  and `ItemInHandRenderer` define which paths supply a living owner.
  `GuiGraphicsExtractor.fakeItem` and block/non-living entity paths do not.

## Ordered Stages, Tests, and Safe Stops

### A. Tint semantic owner

Add exact potion colour mixing, explicit living-owner team context, recursive
inheritance, and focused resolver tests covering precedence, visibility,
weighting, defaults, composites, and bundle-selected leaves.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-item-model item_tint
scripts/cargo-dev.sh test -p bbb-item-model item_runtime
```

This is a buildable safe stop. Existing native callers still pass no team
colour until stage B, so the source default remains visible rather than a
fabricated owner.

### B. Canonical owner projection and native consumers

Expose the world team-colour query and pass it through all implemented
living-owner item paths. Pin ownerless fake/non-living paths with focused tests
so later convenience APIs cannot accidentally inherit the local player's team.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world scoreboard
scripts/cargo-dev.sh test -p bbb-native item_models
scripts/cargo-dev.sh test -p bbb-native hud_item
scripts/cargo-dev.sh test -p bbb-native item_tint
```

This is the integration safe stop. A failure localizes to the world query,
item-runtime tint calculation, or the named native producer.

### C. Closure and gate

Delete the resource-pack tint todo, append the engineering milestone to
`docs/goal-archive.md`, run the repository per-commit checks, commit, and run:

```text
scripts/cargo-dev.sh gate
scripts/audit-p0.sh
```

The final HEAD must be clean and fully gated. Fixes move forward; a failed
stage does not reopen an already accepted resource generation.
