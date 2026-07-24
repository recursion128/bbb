# Potion duration scale ItemStack component round-trip

Status: complete and fully integrated.

## Goal and completion contract

Close all six ItemStack coverage dimensions for Vanilla 26.1
`POTION_DURATION_SCALE` (type 52) as one scalar-default engineering slice.

Completion means the component descriptor owns exact raw-float network
encoding, creative ItemStacks enforce the persistent
`ExtraCodecs.NON_NEGATIVE_FLOAT` domain before framing, and canonical patches
retain the original float bits. The source-derived item prototype catalog must
retain the `0.125F` tipped-arrow and `0.25F` lingering-potion defaults without
hand-maintained item ids or values, publish them through native resource reload
into the world-owned effective ItemStack view, and expose the same effective
value to potion tooltips. Explicit additions override the prototype, explicit
removals fall back to `1.0F`, and absent non-prototype values also use `1.0F`.

Tooltip durations must follow `PotionContents.addPotionTooltip`: the decision
to show a duration uses the unscaled effect (`duration == -1 || duration > 20`);
an infinite duration stays infinite and every displayed finite duration is
formatted from `floor(duration * scale)`. Server-side consumption separately
uses `MobEffectInstance.withScaledDuration`, whose finite nonzero result is
clamped to at least one tick; this client does not predict that application.
The final slice includes focused extraction, world publication/removal,
tooltip, descriptor, patch-framing, creative validation, and
persistent-projection tests before all six ledger cells become `complete`.

## Non-goals

- This slice does not predict server-side potion consumption or mob-effect
  application. The server remains authoritative; the client renders the
  synchronized result through the existing mob-effect path.
- It does not add another scalar component or generalize prototype values into
  an untyped map. The duration scale remains a typed field beside the existing
  typed dye/default owners.
- It does not accept invalid persistent values merely because the network
  codec can carry them. Raw negative zero, negative values, infinities, and
  NaNs round-trip at the descriptor boundary but fail creative untrusted-stack
  validation exactly where Vanilla switches to the persistent Codec.
- It does not reconstruct arbitrary `Items.java` expressions. Only a direct
  `.component(DataComponents.POTION_DURATION_SCALE, <float literal>)` is in
  scope; malformed or duplicate declarations fail catalog construction.

## Ownership and integration boundary

- `bbb-pack::ItemRegistryCatalog` owns CST-derived prototype scale bits and
  resource/protocol-id lookup.
- `bbb-native::ItemRuntimeWorldDefaults` transports those source facts during
  resource reload.
- `bbb-world::ItemProfiles` owns the session-effective prototype value and
  materializes it into `effective_item_stack`; explicit patch additions and
  removals retain precedence.
- `bbb-item-model::NativeItemRuntime` owns tooltip projection and uses the same
  catalog fallback for direct runtime callers that have not passed through the
  world effective-stack boundary.
- `bbb-protocol::packets::data_components::catalog` owns exact network
  encoding; serverbound creative validation owns the stricter persistent
  domain. Sparse coverage overrides remain the only hand-maintained ledger
  input.

## Vanilla 26.1 evidence

- `DataComponents.POTION_DURATION_SCALE` is persistent with
  `ExtraCodecs.NON_NEGATIVE_FLOAT`, network-synchronized with
  `ByteBufCodecs.FLOAT`, and cache encoded.
- `Items.TIPPED_ARROW` declares `POTION_DURATION_SCALE` as `0.125F`;
  `Items.LINGERING_POTION` declares `0.25F`.
- `PotionContents.onConsume` and `PotionContents.addToTooltip` both use
  `components.getOrDefault(POTION_DURATION_SCALE, 1.0F)`.
- `PotionContents.addPotionTooltip` tests `effect.endsWithin(20)` before
  calling `MobEffectUtil.formatDuration`; the formatter preserves infinite
  duration and otherwise uses `Mth.floor(duration * scale)`.
- Server-side `MobEffectInstance.withScaledDuration` leaves infinite and zero
  durations unchanged and otherwise applies
  `max(Mth.floor(duration * scale), 1)`.
- Java `Float.compareTo` supplies the inclusive persistent range check:
  `-0.0F`, negatives, infinities, and NaNs are outside
  `[0.0F, Float.MAX_VALUE]`; positive zero and `Float.MAX_VALUE` are valid.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` length-prefixes each complete
  component value independently.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/{DataComponents,DataComponentPatch}.java`,
`world/item/{Items}.java`, `world/item/alchemy/PotionContents.java`,
`world/effect/MobEffectInstance.java`, and `util/ExtraCodecs.java`.

## Serial stages and validation

1. Extend the generated item catalog with typed scale bits, publish them through
   native reload into world effective patches, and add CST/default/removal
   tests. Focused tests: `bbb-pack`, the world item-component initializer
   module, and native resource reload. Safe stop: prototype scale facts are
   source-derived and world-visible, but no coverage claim is made.
2. Apply effective scales to built-in and custom potion tooltip durations,
   including direct item-runtime prototype fallback. Focused tests:
   `bbb-item-model` tooltip/runtime tests. Safe stop: observable semantics are
   correct while type 52 remains outbound-unsupported.
3. Add exact descriptor encoding, canonical patch preflight, independent
   delimiters, creative persistent validation, and boundary tests. Focused
   tests: `bbb-protocol` descriptor/data-component/serverbound tests and the
   existing native persistent SNBT tests. Safe stop: type 52 is the only newly
   encodable value.
4. Close the six ledger cells, narrow the creative and persistent-SNBT TODOs,
   archive this contract, and add the engineering index entry. Run the catalog
   generator check, every affected focused test, and all per-commit checks.

This crosses pack, world, presentation, native reload, and protocol boundaries.
The preceding potion-effect checkpoint `ba58492eb` passed the full gate before
implementation. Keep the work as one reviewed commit, then run
`scripts/cargo-dev.sh gate`; fix any failure forward before selecting the next
slice.
