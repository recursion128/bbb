# Swing Animation Prototype Default Engineering Contract

Status: completed on 2026-07-30.

## Goal and completion

Close the active Vanilla 26.1 `SWING_ANIMATION` prototype-default gap by
preserving the complete `(type, duration)` value from the item catalog through
the native item runtime and canonical world, then resolving every effective
ItemStack consumer from that same typed default instead of inferring `STAB`
from the presence of a spear-only `KINETIC_WEAPON`.

Completion requires source-derived and override-loaded item catalogs to retain
`NONE`, `WHACK`, and `STAB` defaults with their positive durations; stack
patches and removals to keep Vanilla precedence; local swing timing,
first-person item motion, third-person humanoid attack motion, and held-spear
translation to consume the effective value; focused pack/item-runtime/world/
native tests; removal of the ledger item; archival of this contract; a concise
milestone entry; and a passing full gate at the final head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/item/component/SwingAnimation.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/item/SwingAnimationType.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/item/Item.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/item/ItemStack.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/world/entity/LivingEntity.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/ItemInHandRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/state/ArmedEntityRenderState.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/model/HumanoidModel.java`; and
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/layers/ItemInHandLayer.java`.

`SwingAnimation.DEFAULT` is `(WHACK, 6)`. Its persistent codec accepts a
serialized type and positive duration, defaulting the fields independently;
its network stream carries the generated enum id followed by the raw duration.
`Item.Properties.spear` installs `(STAB, attackDuration * 20)`, but the ordinary
`component` builder can install any of the three types on another prototype.

`ItemStack.getSwingAnimation` resolves the effective component or the default.
Thus an explicit stack patch wins over the prototype, removing the component
falls back to `(WHACK, 6)`, and a prototype with no component does the same.
`LivingEntity` consumes the duration. `ItemInHandRenderer`,
`ArmedEntityRenderState`, `HumanoidModel`, and `ItemInHandLayer` consume the
type for first-person `NONE`/`WHACK`/`STAB`, third-person attack pose, and the
STAB held-item translation.

## Ownership and frozen interfaces

- `bbb-pack::ItemRegistryCatalog` replaces its duration-only default table with
  one typed `ItemSwingAnimation` table. The public query returns the complete
  value. Generated `spear(...)` declarations produce `STAB`; explicit
  `SWING_ANIMATION` constructors resolve their generated enum constant and
  positive duration. The JSON override schema emits `{type, duration}` and
  accepts the old duration-only field as a legacy spear/STAB input.
- `bbb-item-model::NativeItemRuntime` projects protocol-id →
  `SwingAnimationSummary` without handwritten ids or resource-name inference.
- `bbb-world` owns the published typed prototype map and one effective-stack
  resolver. Empty stacks, missing prototypes, and explicit removals return
  `(WHACK, 6)`; an explicit patch wins; non-positive network durations retain
  the existing bounded fallback to `6` without changing their valid type.
  Local swing duration and native entity projection consume this resolver.
- `bbb-native` first-person projection reads the typed runtime default when a
  stack patch is absent. Third-person entity projection reads the canonical
  world resolver for both hands and derives its existing renderer-facing
  `is_stab` / `is_none` flags. Renderer model APIs do not change.
- Resource reload replaces the typed table atomically with the rest of the
  item-runtime defaults. No renderer-owned or independently mutable prototype
  cache is introduced.

## Bounds and fallback rules

1. Stack patch removal has priority over both a patch payload and a prototype
   default, yielding Vanilla's `(WHACK, 6)` fallback.
2. An explicit stack payload has priority over the prototype. Unknown network
   type ids retain the protocol decoder's existing `NONE` normalization.
3. Persistent/generated prototype durations must be positive. Override load
   rejects malformed type names and non-positive structured durations.
4. The legacy duration-only override alias is accepted only as the old
   spear-shaped `STAB` representation; new serialization always emits the
   typed field.
5. Missing resource ids, missing runtime/catalog residency, empty stacks, and
   items without the component fail to `(WHACK, 6)`, never to spear inference.
6. `KINETIC_WEAPON`, `PIERCING_WEAPON`, use-item animation, attack range,
   damage/knockback authority, and renderer geometry remain unchanged.

## Non-goals

- This slice does not add custom item registration, an in-game item editor, or
  a second item-prototype schema.
- It does not change the `SWING_ANIMATION` packet codec or persistent dialog
  codec, both of which already preserve the complete value.
- It does not simulate server-authoritative attack results or change swing
  packet ordering.
- It does not refactor the broader item registry, resource reload transaction,
  humanoid model hierarchy, or first-person rendering pipeline.

## Serial stages and verification

1. **Typed catalog/runtime projection.** Add the typed pack value, exact
   generated/override parsing and protocol-id runtime projection. Prove all
   Vanilla spears plus custom `NONE`/`WHACK`/`STAB`, malformed bounds, legacy
   override compatibility, and serialized shape. Run
   `scripts/cargo-dev.sh test -p bbb-pack swing_animation` and
   `scripts/cargo-dev.sh test -p bbb-item-model swing_animation`.
2. **Canonical effective value and presentation.** Replace the duration-only
   world map, centralize patch/removal/default precedence, and route local
   timing plus first-/third-person consumers through the typed value. Prove
   prototype types, patch precedence, removal fallback, both hands, and
   reload replacement with
   `scripts/cargo-dev.sh test -p bbb-world swing_animation` and
   `scripts/cargo-dev.sh test -p bbb-native swing_animation`.
3. **Closeout.** Re-run all focused families, remove the ledger item, archive
   this contract, append the milestone index, run the standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`35722e575925f94091c5dcf5842bdba31ded75b3`. The final full gate is the
engineering checkpoint. No playable gate is required because the slice
changes no packet, input-command, world-mutation, or control-command boundary.

The contract, typed catalog/runtime projection, and effective world/native
consumer stages are the clean commits `b2e19b868`, `1aeac2e42`, and
`a5237afb3`.

## Failure localization and safe stops

Catalog failures compare the generated source expression, typed default,
serialized override, and protocol-id projection before world code. World
failures compare empty/missing/removal/patch/prototype precedence and sanitized
duration before native model selection. Native failures compare first-person
and both-hand third-person type selection before renderer transforms.

The clean contract commit and the typed catalog/runtime commit are safe stops.
Any focused or gate failure is fixed forward before another slice; history is
not rewritten.
