# Damage-type item component engineering contract

## Goal and completion

Close `minecraft:damage_type` as one item-component vertical slice:

- preserve the network holder id in canonical `DataComponentPatchSummary`;
- resolve spear prototype defaults through the existing frozen
  registry-dependent item-default owner;
- expose the effective named holder to Vanilla's generic
  `minecraft:component` item-model select property;
- encode the exact holder-id stream value and creative-slot patch; and
- mark all six item-component coverage dimensions complete with focused tests.

The slice is complete when explicit patches, removals, spear defaults, custom
session registry order, item-model selection, persistent SNBT, descriptor
round-trip, and creative packet framing are covered and the final HEAD passes
the full gate.

## Non-goals

- Client-side damage application or prediction. Vanilla chooses the damage
  source in server-side `ItemStack.getDamageSource` attack handling.
- A second registry cache, fixed holder ordinal, or item-model-specific
  registry owner.
- Typed decoding of the full `DamageType` registry payload, which is already
  owned by the registry ledger and is not required to retain or select its
  resource identity.

## Ownership and integration contract

- `bbb-pack` derives spear's `minecraft:spear` holder key from
  `Item.Properties.spear` and stores it as a
  `RegistryDependentItemComponentInitializer` rule.
- `bbb-world` resolves that rule against the candidate
  `FrozenRegistryAccess`, commits the holder id atomically with the other item
  defaults, and keeps explicit patch/removal precedence in
  `effective_item_stack`.
- `bbb-item-model` receives the current damage-type resource-id order beside
  the existing registry-key context. Its component-select property resolves
  explicit holder ids through that slice and uses the pack-derived spear key
  for an unpatched prototype.
- `bbb-native` only projects the world-owned registry order into existing item
  rendering entry points. It does not retain another generation or holder map.
- `bbb-protocol` owns raw holder-id decode/encode and creative patch framing.
  Outgoing summaries require a non-negative holder id; the receiving server's
  registry-aware validated item codec remains authoritative for membership.

Authoritative Vanilla sources:

- `DataComponents.DAMAGE_TYPE` and `DamageType.STREAM_CODEC`;
- `Item.Properties.spear`;
- `ItemStack.getDamageSource`;
- `ComponentContents.get` and its component-type codec dispatch; and
- `ServerboundSetCreativeModeSlotPacket.STREAM_CODEC`.

## Serial stages and verification

1. Registry default foundation (`bbb-pack` + `bbb-world`): add the initializer,
   source extraction, frozen-registry resolution, precedence tests, and
   catalog guards. Focused tests cover generated spear membership, custom
   registry reordering, missing holder rejection, patch override, and removal.
2. Presentation and protocol closeout (`bbb-item-model` + `bbb-native` +
   `bbb-protocol` + ledgers): thread the current damage-type keys, implement
   component-select defaults/patches/removals, add the descriptor and creative
   encoder tests, regenerate the matrix, and update unsupported features.

Each stage ends in a buildable reviewed commit after the per-commit checks.
The full gate runs at final closeout (and immediately after any abnormal
focused-test failure). If stage 2 cannot preserve the single world registry
owner without broad API divergence, stage 1 is the safe stopping point and the
remaining presentation contract stays explicitly open.
