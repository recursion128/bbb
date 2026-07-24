# Adventure-mode predicate item component engineering contract

## Goal and completion

Close `minecraft:can_place_on` and `minecraft:can_break` as the shared
`AdventureModePredicate` item-component vertical slice:

- retain the complete `BlockPredicate` list from persistent and network input;
- encode the exact `AdventureModePredicate.STREAM_CODEC`, including nested exact
  component values and partial predicate payloads;
- evaluate the effective selected-stack predicate against the current world
  block with Vanilla holder-set, state-property, and NBT semantics;
- apply the result to adventure-mode block breaking and block-outline
  visibility; and
- mark all six item-component coverage dimensions complete with focused tests.

The slice is complete when direct block holders, dynamic block tags, absent
block constraints, exact and ranged state matchers, partial compound/list NBT,
ignored block-entity component matchers, explicit component removal, nested
network component values, descriptor round-trip, creative packet framing,
adventure block breaking, and block-outline visibility are covered and the
final HEAD passes the full gate.

## Non-goals

- Server-authoritative block placement, breaking, or block-entity mutation.
- Suppressing `ServerboundUseItemOnPacket` when `can_place_on` does not match.
  Vanilla `MultiPlayerGameMode.useItemOn` sends the packet even when local
  `ItemStack.useOn` returns `PASS`; the predicate only gates the local item
  behavior.
- Evaluating `DataComponentMatchers` for the adventure item consumers.
  Vanilla calls `BlockPredicate.matches(BlockInWorld)`, which checks state and
  NBT but intentionally omits the server-level block-entity component branch.
- Reconstructing block-entity data that the server did not publish in its
  update tag.
- Changing spectator outline/menu-provider behavior or implementing unrelated
  local block-use simulation.

## Ownership and integration contract

- `bbb-protocol` keeps `AdventureModePredicateDataSummary` as the canonical
  structured value. Its descriptor encodes the shared type for both component
  ids, writes exact component predicate values without item-patch length
  delimiters, preserves partial predicate NBT, and rejects invalid counts,
  duplicate maps, mixed holder-set forms, invalid NBT roots, and excessive
  recursive nesting atomically.
- `bbb-world` owns predicate evaluation. Direct block holders and tags resolve
  through the active `RegistrySet`; state range comparisons use the Vanilla
  block-state property's declared value order; NBT follows
  `NbtUtils.compareNbt(expected, actual, true)` against the client block
  entity's update data plus Vanilla full-metadata fields.
- `bbb-native` asks the world owner whether the selected main-hand stack can
  break or place on the hit block. Adventure-mode destroy input requires
  `can_break`; adventure outlines require `can_break || can_place_on`.
  Survival/creative paths remain unchanged.
- Existing tooltip and persistent-SNBT owners remain the presentation and
  command projection paths. No parallel predicate representation is added.

Authoritative Vanilla sources:

- `DataComponents.CAN_PLACE_ON` / `CAN_BREAK`;
- `AdventureModePredicate.CODEC` / `STREAM_CODEC` / `test`;
- `BlockPredicate.CODEC` / `STREAM_CODEC` / `matches(BlockInWorld)`;
- `StatePropertiesPredicate` exact/ranged matcher stream codecs;
- `NbtPredicate` and `NbtUtils.compareNbt`;
- `ItemStack.canPlaceOnBlockInAdventureMode`,
  `ItemStack.canBreakBlockInAdventureMode`, and `ItemStack.useOn`;
- `Player.blockActionRestricted`;
- `GameRenderer.shouldRenderBlockOutline`; and
- `ServerboundSetCreativeModeSlotPacket.STREAM_CODEC`.

## Serial stages and verification

1. Protocol reverse codec (`bbb-protocol`): add bounded recursive decode,
   descriptor-owned reverse encoding, nested exact/partial matcher tests,
   atomic rejection tests, delimited patch round-trip, creative exact bytes,
   and untrusted persistent-range validation. Focused commands:
   `scripts/cargo-dev.sh test -p bbb-protocol adventure_mode_predicate` and
   `scripts/cargo-dev.sh test -p bbb-protocol set_creative_mode_slot`.
2. World and native consumers (`bbb-world` + `bbb-native`): add one evaluator
   for holder/tag/state/NBT semantics, selected-stack effective-component
   precedence, adventure destroy restriction, and outline visibility. Focused
   commands: `scripts/cargo-dev.sh test -p bbb-world adventure_mode` and
   `scripts/cargo-dev.sh test -p bbb-native adventure_mode`.
3. Closeout: update the creative-encoder TODO, sparse item-component coverage
   evidence, generated matrix, and engineering archive index; run their
   focused checks and the final full gate.

Each stage ends in a buildable reviewed commit after the per-commit checks.
The already-gated pre-start HEAD is the high-risk baseline; the full gate runs
at final closeout, and immediately after any abnormal focused-test failure.
If stage 2 reveals that canonical block-entity update data cannot be exposed
without a new serialization owner, stage 1 is the safe stopping point: retain
the protocol commit and leave NBT gameplay consumers and their coverage cells
open rather than fabricating unavailable state.
