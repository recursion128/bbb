# Foundational ItemStack component round-trip

## Goal and completion contract

Close all six item-component coverage dimensions for the four foundational
Vanilla 26.1 stack invariants:

- `MAX_STACK_SIZE`;
- `MAX_DAMAGE`;
- `DAMAGE`; and
- `UNBREAKABLE`.

Completion means their generated catalog descriptors own exact network value
encoding as well as typed decoding; `DataComponentPatchSummary` can validate
and encode a canonical patch containing these additions and arbitrary valid
removals; and `ServerboundSetCreativeModeSlotPacket` emits Vanilla's
length-delimited untrusted ItemStack form. The encoder must preserve summary
order, reject duplicate or overlapping map keys, reject a missing or dangling
summary field, and fail before emitting bytes for any unsupported added value.
The creative-slot boundary must additionally reject the persistent integer
ranges that Vanilla's validated untrusted decoder rejects.

Existing canonical fields, persistent SNBT projection, inventory behavior,
item-model predicates, durability presentation, and unit-component consumers
must have named focused-test evidence before any ledger cell becomes
`complete`.

## Non-goals

- This slice does not encode the other 106 data-component values.
- It does not mark `SERVERBOUND_SET_CREATIVE_MODE_SLOT` complete or remove its
  remaining component-rich encoding gap.
- It does not create a second component catalog, parallel type-id switch, or a
  generic dynamic value representation.
- It does not add `ItemStack.validateStrict` policy that Vanilla's
  `validatedStreamCodec` does not execute while encoding. In particular, the
  generic `VAR_INT` value encoder remains able to represent every `i32`; the
  creative packet boundary applies only the receiving persistent-codec ranges.
- It does not change default item components, component patch sanitization, or
  local inventory prediction for unsupported component values.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns the optional network
  encode callback beside each generated metadata/decode descriptor. Only the
  four selected descriptors gain callbacks in this slice.
- `bbb-protocol::packets::data_components` owns canonical patch-shape
  validation, descriptor dispatch, wire ordering, and the per-added-value
  length prefix used by `DataComponentPatch.DELIMITED_STREAM_CODEC`.
- `bbb-protocol::packets::serverbound` owns creative ItemStack validation and
  invokes that delimited patch encoder after count and item-holder encoding.
- `bbb-native`, `bbb-world`, and `bbb-item-model` remain the existing owners of
  persistent projection and observable behavior; this slice audits and tests
  those paths instead of introducing parallel consumers.
- The sparse item-component coverage override is the only hand-maintained
  ledger input. Each of the four rows may become complete only with exact
  implementation and test symbols.

## Vanilla 26.1 evidence

- `DataComponents.java` registers `MAX_STACK_SIZE`, `MAX_DAMAGE`, and `DAMAGE`
  with `ByteBufCodecs.VAR_INT`; their persistent codecs respectively require
  `1..=99`, a positive integer, and a non-negative integer.
  `UNBREAKABLE` uses zero-byte `Unit.STREAM_CODEC` and persistent `Unit.CODEC`.
- `DataComponentPatch.createStreamCodec` writes positive and negative counts,
  then positive type/value entries followed by negative type-only entries.
  `DELIMITED_STREAM_CODEC` wraps every positive value codec independently in
  `ByteBufCodecs.registryFriendlyLengthPrefixed`.
- `ByteBufCodecs.lengthPrefixed` encodes a value into a scratch buffer, writes
  its VarInt byte length, then copies the exact value bytes. Thus
  `UNBREAKABLE` is encoded as its type followed by a zero length.
- `ServerboundSetCreativeModeSlotPacket.STREAM_CODEC` writes a short slot then
  `ItemStack.validatedStreamCodec(ItemStack.OPTIONAL_UNTRUSTED_STREAM_CODEC)`.
  On decode, that wrapper projects a non-empty stack through `ItemStack.CODEC`,
  enforcing the persistent component ranges and the stack count range
  `1..=99`; it does not call `ItemStack.validateStrict`.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`core/component/DataComponentPatch.java`, `network/codec/ByteBufCodecs.java`,
`world/item/ItemStack.java`, and
`network/protocol/game/ServerboundSetCreativeModeSlotPacket.java`.

## Serial stages and validation

1. Extend the catalog descriptor with optional encode ownership, implement the
   four exact value codecs, then add canonical patch preflight and delimited
   framing. Focused tests prove raw value bytes, exact added/removal order,
   zero-byte Unit framing, and every malformed-summary rejection. Safe stop:
   no caller is wired until these tests pass.
2. Integrate the preflighted encoder into creative-slot ItemStacks and enforce
   the receiving persistent ranges, including count `1..=99`. Focused packet
   tests prove exact bytes, decode each bounded value slice back through the
   descriptor, and retain precise rejection for an unsupported addition. Safe
   stop: retain the prior componentless path if packet integration fails.
3. Audit canonical, persistent, and observable consumers; add only missing
   focused assertions; close the four six-dimensional ledger rows, narrow the
   creative-slot TODO, and archive this contract. Run protocol, native, world,
   and item-model focused tests, the per-commit checks, and review the complete
   diff.

This is one high-risk protocol engineering milestone. The integrated commit
must pass the full repository gate before another slice starts. Failures remain
localized by stage-specific focused tests; implementation is fix-forward and
the clean pre-slice commit is the rollback boundary.
