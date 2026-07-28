# Targeted Local-Use Prediction Closure

This document freezes the remaining engineering contract for Vanilla 26.1
targeted local-use prediction and first-person success presentation. Work starts
from full-gated commit `310db040b`.

## Goal and Completion Standard

Close the local client path selected by `Minecraft.startUseItem` for entity,
block, and empty-space use:

- every generated `Entity.interact`, `Mob.mobInteract`, item interaction,
  `Block.useItemOn`, and `Block.useWithoutItem` owner has either an exact
  source-backed client result/mutation or an explicit source-backed
  server-authoritative/no-client-observable disposition;
- successful `interact`, `use_item_on`, and `use_item` commands use Vanilla's
  exact client swing source and hand-item presentation rules;
- first-person item replacement and use feedback retain the renderer-owned
  visible stack and interpolate the hand height using
  `ItemInHandRenderer.tick` / `itemUsed`, including the generated client-item
  swap properties and the damage component's ignore-swap-animation rule; and
- packet/world/native reverse tests prove the generated owner catalogs are
  exhaustively consumed, while server equipment, inventory, entity, and block
  corrections remain authoritative.

The engineering slice closes only when the three serverbound coverage rows no
longer carry local-prediction `partial` cells, the targeted-use TODO is removed,
focused tests and the full gate pass at current HEAD, and the milestone is
indexed in `docs/goal-archive.md`.

## Non-Goals

- Do not simulate server AI, breeding, taming probability, entity damage,
  spawning, loot, navigation, or authoritative mob inventories.
- Do not invent inventory consumption, equipment changes, entity data, or
  block-state changes that Vanilla leaves to the server. Existing prediction
  sequence acknowledgements and server corrections stay canonical.
- Do not implement the creative inventory screen, general GUI configuration,
  missing special item renderers, or unrelated third-person item animation.
- Do not duplicate generated Java-owner catalogs in handwritten membership
  tables or infer behavior from entity resource names.

## Owners and Frozen Boundaries

- `bbb-protocol` owns the generated item, block, and fixed-entity Java-owner
  catalogs. Handwritten prediction dispatch consumes those enums exhaustively;
  generator/source-extraction changes are separate high-risk checkpoints.
- `bbb-world::client::inventory::use_prediction` owns pure local interaction
  selection, client-observable canonical mutations, and the
  `LocalInteractionSuccess` contract. Its `was_item_interaction` bit means the
  successful block result came from the held-item callback rather than the
  block fallback; it is not an inventory-authority signal.
- `bbb-native::input::mouse` preserves Vanilla call order, captures the
  pre-interaction held stack needed by `Minecraft.startUseItem`, sends the
  serverbound command once, and applies success presentation once.
- `bbb-native::item_models` owns renderer-input hand presentation. Mutable
  state retains at most two reference-counted visible stack summaries plus
  previous/current hand heights; immutable publications clone only those cheap
  handles. World state does not acquire renderer-owned animation fields.
- `bbb-item-model` projects `ClientItem.Properties` and answers stack model,
  hand-animation-on-swap, and swap-animation-scale queries from the accepted
  resource generation. It does not own animation time.
- `bbb-renderer` consumes already-resolved first-person stack/height inputs and
  remains submission-only.

## Vanilla 26.1 Evidence

The main agent verifies local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each stage. Primary anchors are:

- `Minecraft.startUseItem` and `MultiPlayerGameMode.interact`,
  `performUseItemOn`, and `useItem` for result order, swing source, count
  snapshots, infinite-material behavior, and `itemUsed` calls;
- `Entity.interact`, `Mob.mobInteract`, `Animal`, `AgeableMob`,
  `TamableAnimal`, `AbstractHorse`, and every generated concrete
  `mobInteract` declaration for client-side return values and observable
  mutations;
- `ItemInHandRenderer.tick`, `itemUsed`, and first-person render entry points
  for visible-stack replacement, height clamping, hands-busy lowering, attack
  scale, and partial-tick interpolation;
- `ItemStack.matchesIgnoringComponents`, `DataComponentType` /
  `DataComponents`, `ClientItem.Properties`, and `ItemModelResolver` for
  replacement suppression and resource-defined scale; and
- all generated block/item owner source methods already referenced by the
  existing prediction modules.

Historical ledger wording and generated owner names are routing evidence, not
behavioral evidence. Each implemented result or mutation is checked against the
current method body and inherited fallback chain.

## Ordered Stages and Focused Tests

### A. Shared mob parents and client state

Inventory all generated `Mob.mobInteract` owners against the Vanilla inheritance
chain. Add only the lightweight entity fields that the client owns and that
subsequent local decisions can observe. Implement shared `Mob`, `Animal`,
`AgeableMob`, tameable, horse, and trader fallbacks before concrete subclasses.
Unknown/missing packet facts fail closed without speculative mutation.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world local_mob_interact
scripts/cargo-dev.sh test -p bbb-world local_entity_interact
scripts/cargo-dev.sh test -p bbb-world use_prediction
```

### B. Specialized generated owners

Implement every remaining concrete owner in generated order, reusing shared
shears, leash, food, mount, bucket, and equipment helpers. Add a reverse guard
that iterates every fixed entity row and rejects any generated owner left on the
generic unsupported branch. A source-backed server-only branch returns the
exact client result without fabricating server state.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world local_mob_interact
scripts/cargo-dev.sh test -p bbb-protocol generated_entity
scripts/vanilla-source-tools.sh python -m unittest scripts.tests.test_entity_catalog
```

### C. First-person success presentation

Project client-item swap properties into `NativeItemRuntime`, add the native
hand-presentation owner, and feed its visible stacks and interpolated inverse
heights into item and arm extraction. Apply success feedback exactly:

- entity interaction: client swing only when requested, never `itemUsed`;
- block interaction: client swing when requested, then `itemUsed` only when the
  pre-use stack was nonempty and its count changed, or the player has infinite
  materials;
- empty-space item use: client swing when requested and always `itemUsed`.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-item-model client_item
scripts/cargo-dev.sh test -p bbb-native first_person
scripts/cargo-dev.sh test -p bbb-native mouse
```

### D. Coverage closeout

Re-audit all generated item, block, and entity owners, close the three
serverbound packet rows, remove the targeted-use TODO, regenerate or validate
affected ledgers, and append the engineering milestone index.

Run:

```text
scripts/cargo-dev.sh gate
scripts/playable-gate.sh
```

## Checkpoints, Failures, and Safe Stops

The baseline is fully gated. The contract, shared-parent closure, specialized
owner closure, first-person presentation, and coverage closeout are separate
buildable commits. Run the full gate after at most four commits, before and
after any generator change, when crossing into the renderer-input state owner,
and at final closeout.

Focused failures are fixed forward; `batch-gate` is used only when the
introducing commit is unclear. Safe stops are a clean contract checkpoint, a
complete shared-parent layer with prior unsupported behavior preserved for
specialized owners, an exhaustive specialized-owner checkpoint, a fully wired
presentation state with all callers migrated, or a documented external blocker
with prior behavior intact. No predicted world mutation lands without its
source-backed result test, and no renderer-input state lands without its
published consumer in the same checkpoint.
