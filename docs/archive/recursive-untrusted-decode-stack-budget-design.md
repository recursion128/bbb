# Recursive Untrusted Decode Stack-Budget Closure

Status: completed on 2026-07-27.

This document freezes the engineering contract for the remaining
byte-denominated recursion guards in the protocol ledger. Work starts from
fully gated commit `7980738db`.

## Goal and Completion Standard

Close the three remaining recursive untrusted semantic-decode families named
in `docs/unsupported-features.md`:

- generic Component trees, including the full trusted network-NBT decode,
  structural validation, styled-run projection, owner-payload traversal, and
  canonical JsonOps-length projection;
- flattened SNBT compound/value parsing; and
- persistent HolderSets whose direct element codec is itself recursive.

Every accepted maximum-depth fixture must execute on an explicitly
production-sized thread and record actual stack-address extremes. A
byte-denominated assertion must retain caller headroom and fail if later
debug-profile frame growth makes the semantic constant unsafe. The exact
boundary and first rejected depth must remain deterministic, and malformed
input must not poison a later decode.

The HolderSet audit is part of completion, not an assumption. Vanilla's
ordinary network HolderSet and persistent scalar/list/tag forms do not recurse.
The client-observable recursive case is `Dialog.LIST_CODEC`: a direct dialog
inside a dialog-list HolderSet can contain another dialog list or a component
action that resolves another dialog. Its depth must remain governed by the
existing dialog semantic limit and its existing byte budget. Completion
requires an exact direct-HolderSet boundary fixture in addition to the existing
show-dialog reference-walk fixture.

The slice is complete when all three families have accepted-boundary,
over-boundary, recovery, and measured-stack evidence; the broad ledger todo can
be deleted; focused tests and the final full gate pass on a clean committed
tip.

## Non-Goals

- Changing Vanilla wire fields, packet ids, registry ordering, or Component,
  SNBT, Dialog, or HolderSet value semantics.
- Raising the current bbb defensive semantic limits merely because Vanilla's
  generic codecs permit deeper values.
- Adding a second NBT tree, Component schema, dialog resolver, or HolderSet
  resolver.
- Generalizing this slice to the independent chunk/block-entity raw-NBT skip
  path, tag-query raw-NBT skip path, recursive ItemStack templates, dialog
  shapes other than the HolderSet proof, or non-decode recursive consumers.
  Recursive ItemStack templates and the show-dialog walk already have their
  own byte-budget guards.
- Treating ordinary network or persistent HolderSet loops as recursive or
  adding a meaningless stack probe to them.
- Using a larger production worker stack as a substitute for a measured
  maximum-depth budget.

## Ownership and Frozen Boundaries

- `bbb-protocol::nbt_summary` remains the sole network-NBT and flattened-SNBT
  tree owner. Component tests enter through the normal network decoder; the
  standalone SNBT test enters through `parse_snbt_compound` /
  `parse_snbt_prefix`.
- `bbb-protocol::component` owns Component structural validation, payload
  visitation, styled-run projection, and canonical JsonOps projection. Test
  probes may be shared inside this semantic owner, but no test-only state is
  exposed from the crate.
- The existing Component nesting limit remains one shared constant. All
  recursively reachable semantic passes must either stay within its measured
  byte budget or be converted to an equivalent iterative walk before the
  ledger item closes.
- `bbb-world::client::dialog` retains `MAX_DIALOG_SEMANTIC_DEPTH`,
  `STACK_PROBE_EXTREMES`, registry-aware direct/reference resolution, and the
  existing 4 MiB budget on its explicit 8 MiB test thread. The new HolderSet
  fixture reuses that owner rather than adding a protocol-level holder budget.
- Ordinary `ByteBufCodecs.holderSet` and `HolderSetCodec` scalar/list/tag
  resolution stays unchanged because those shapes have bounded iteration but
  no recursive descent unless the supplied direct element codec is recursive.

## Vanilla 26.1 Evidence

- `ComponentSerialization.CODEC` is declared with `Codec.recursive`. Its root
  alternatives are a string, a non-empty list of Components, or a full
  Component. Recursion also occurs through `extra`, translatable arguments,
  separator/fallback Components, `show_text`, and optional `show_entity.name`.
  Network stream codecs delegate to this codec through NBT operations.
- `TagParser` delegates flattened SNBT to `SnbtGrammar`; compounds, lists,
  arrays, and builtin arguments form the nested value grammar. Vanilla uses
  Java UTF-16 strings. bbb's depth 64 is a defensive client limit rather than
  a claimed Vanilla protocol limit.
- `HolderSetCodec` accepts a tag or homogeneous direct element list when
  registry-aware, and a direct element list without a registry.
  `ByteBufCodecs.holderSet` similarly decodes either a tag or a flat holder-id
  list. Neither codec recursively nests HolderSets by itself.
- `Dialog.LIST_CODEC` is
  `RegistryCodecs.homogeneousList(Registries.DIALOG, DIRECT_CODEC)`, and
  `DialogListDialog` stores that set in its `dialogs` field. Because
  `Dialog.DIRECT_CODEC` dispatches back to dialog codecs, direct dialog-list
  entries are the concrete recursive HolderSet surface.

Authoritative sources are under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/`, especially
`network/chat/ComponentSerialization.java`, `nbt/TagParser.java`,
`nbt/SnbtGrammar.java`, `resources/HolderSetCodec.java`,
`network/codec/ByteBufCodecs.java`, `server/dialog/Dialog.java`, and
`server/dialog/DialogListDialog.java`.

## Serial Stages and Focused Tests

### A. Component end-to-end budget

Add test-only address probes to every recursive phase reached by a maximum-depth
network Component. Prove the exact accepted/rejected boundary, full decode and
styled projection on a 2 MiB thread, owner-payload traversal, canonical
projection, and recovery after failure. If any pass exceeds the fixed headroom
budget, make that pass iterative without changing semantic order or results.

Focused commands:

```text
scripts/cargo-dev.sh test -p bbb-protocol component_stack_budget
scripts/cargo-dev.sh test -p bbb-protocol component
```

### B. SNBT and recursive HolderSet budgets

Measure maximum-depth flattened SNBT parsing on a 2 MiB thread and pin its exact
boundary plus recovery. Add a depth-64 direct dialog-list HolderSet fixture,
the first rejected nesting, and measured stack use through the existing dialog
budget owner. Prove network and ordinary persistent HolderSet forms remain
flat by source-backed structural assertions rather than new runtime state.

Focused commands:

```text
scripts/cargo-dev.sh test -p bbb-protocol snbt_stack_budget
scripts/cargo-dev.sh test -p bbb-world dialog_holder_set_stack_budget
scripts/cargo-dev.sh test -p bbb-world dialog --lib
```

This stage is the cross-crate high-risk checkpoint and ends with a full gate.

### C. Closeout

Delete only the completed broad stack-budget todo, archive this contract with a
completion status, add one milestone index entry, run documentation checks,
commit, and run the final full gate and P0 audit.

## Checkpoints, Failure Localization, and Safe Stops

The contract, Component budget, SNBT/HolderSet budget, and documentation
closeout are separate buildable commits. The baseline is already fully gated.
Run a full gate after stage B, by the fourth commit, and at final HEAD.

Each probe reports the semantic family, accepted depth, measured bytes, and
budget. A focused failure identifies the owning phase; do not use
`batch-gate` unless the introducing commit remains unclear. Fix failures
forward.

Safe stops are a clean contract commit; Component evidence with all prior valid
Components unchanged; or complete SNBT evidence plus the existing dialog
budget still green. The ledger todo remains until all three families and the
final gate are complete.
