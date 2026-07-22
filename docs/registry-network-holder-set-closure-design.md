# Network Holder-Set Identity Closure

This document freezes the engineering contract for closing the ten remaining
Vanilla 26.1 `ByteBufCodecs.holderSet` call sites in
`docs/registry-holder-consumer-coverage-26.1.json`. Work starts from fully
gated commit `f5901e57f`.

## Goal and Completion Standard

Close the `network-holder-set-or-tag` coverage family without treating a raw
VarInt list or a generic resolver API as proof. The fixed inventory is:

- `BlockPredicate` and `Tool.Rule` over the immutable block registry;
- `Ingredient` (required and optional) and `Repairable` over the immutable item
  registry;
- `RemoveStatusEffectsConsumeEffect` over the immutable mob-effect registry;
- `Equippable.allowedEntities` over the immutable entity-type registry; and
- `BlocksAttacks`, `DamageReduction`, and `DamageResistant` over the
  synchronized damage-type registry.

Direct holders must reject identities absent from the registry view used by
Vanilla's `RegistryFriendlyByteBuf`. Named sets must retain the normalized tag
and resolve only through the same frozen registry revision. Built-in registry
ids may use generated fixed catalogs; synchronized damage types must use the
server holder order and tags. No custom insertion, reordering, missing tag,
unresolved holder, or configuration re-entry may reinterpret one identity as
another.

The slice closes only when all ten generated ledger rows are `complete`, their
semantic consumers have direct and tag-form evidence, reverse tests keep the
call-site inventory exact, focused tests pass, and the applicable checkpoint
full gate is green.

## Non-Goals

- The separate `Instrument` named/direct holder, `TagKey.streamCodec`, and all
  persistent `Codec` holder families remain outside this slice.
- Server-authoritative damage reduction, item repair, equipment eligibility,
  advancement predicates, and consume-effect execution are not simulated
  merely to close holder identity. Existing client-observable recipe matching,
  item use pose, tooltip/debug reconstruction, and canonical component state
  remain the consumers.
- `bbb-protocol` does not acquire mutable world/session registry ownership, and
  the renderer does not resolve holder ids or tags.
- Missing or malformed holder sets do not receive bootstrap-ordinal,
  resource-name, empty-set, or first-entry fallbacks.

## Owners and Frozen Boundaries

- `bbb-protocol` owns the exact count/tag wire shape, allocation bounds, and
  validation of direct holders whose registry identity is generated and
  immutable (`item`, `mob_effect`, and `entity_type`). Block identity remains
  with the generated block-state registry in `bbb-world`; synchronized
  `damage_type` identity cannot be validated without the frozen world view.
- `bbb-world::registries` owns holder/tag lookup, unresolved-tag diagnostics,
  synchronized holder order, and revision/session lifetime. Recipe application
  validates its `Ingredient` sets atomically before replacing canonical state.
- Canonical item-stack/component owners retain the decoded shape. Every
  client-observable resolver or persistent reconstruction validates it through
  the generated built-in catalog or current frozen registry access; a consumer
  that lacks the required context fails closed.
- `bbb-native` may format persistent component SNBT only after those typed
  lookups succeed. It does not infer resources from numeric values.
- Coverage selectors may split the current broad family only by the ten frozen
  source paths/resource ids; no unrelated holder family changes status here.

## Vanilla 26.1 Evidence

`ByteBufCodecs.holderSet` decodes `VarInt - 1`: `-1` selects a named tag from
the packet buffer's `RegistryAccess`; any other nonnegative count decodes each
reference through `Registry.asHolderIdMap().byIdOrThrow`. The ten declarations
are the exact calls listed above. `Ingredient` additionally rejects direct air
and non-optional empty sets, while the optional stream maps an empty direct set
to `Optional.empty`.

`RegistryDataCollector` combines configuration contents and tags before
freezing the remote layer. Dynamic damage-type holders therefore use server
order; static block/item/mob-effect/entity-type tags resolve against the
built-in layer. Play `UpdateTags` replaces tags against the already frozen
access. These facts are rechecked from local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each correction.

## Ordered Stages and Focused Tests

### A. Fixed-registry direct holders

Route item, mob-effect, and entity-type holder-set references through their
generated catalog validators, including recipe `Ingredient`; preserve the
bounded count/tag wire shape. Add invalid-id, negative-id, empty required
ingredient, optional-empty, and air rejection coverage where Vanilla enforces
it.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-protocol holder_set
scripts/cargo-dev.sh test -p bbb-protocol ingredient
```

### B. Frozen registry consumers

Validate block and item recipe/tag consumers against the committed built-in
view, and prove synchronized damage-type direct/tag forms use custom server
order. Missing tags and unresolved direct holders fail without replacing prior
canonical recipe state or emitting a misleading persistent value. Re-entry
must clear or revision-isolate affected state.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world holder_set
scripts/cargo-dev.sh test -p bbb-world update_recipes
scripts/cargo-dev.sh test -p bbb-native holder_set
```

### C. Coverage closeout

Split or replace the broad coverage selector with source-backed complete
families for all ten call sites, regenerate the bootstrap catalog outputs, and
add a reverse assertion that none of those calls can return to partial or
deferred without a deliberate ledger change.

Focused tests:

```text
scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check
scripts/cargo-dev.sh test -p bbb-protocol registry_holder_coverage
```

## Checkpoints, Failures, and Safe Stops

The baseline is fully gated. The contract, fixed-registry validation, frozen
consumer closure, and coverage closeout are separate buildable commits. Run a
full gate by the fourth commit and at final closeout. Failures are fixed
forward; `batch-gate` is reserved for unclear introducing commits.

Safe stops are a clean contract commit, strict built-in decoding with existing
valid packets unchanged, registry-aware consumers with old canonical state
preserved on invalid input, or a documented source-backed blocker. No coverage
row becomes complete before its consumer and custom-order/tag tests land.
