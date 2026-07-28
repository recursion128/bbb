# Persistent Holder-Set Coverage Closure

This document freezes the engineering contract for the 48 remaining Vanilla
26.1 `RegistryCodecs.homogeneousList` call sites in the
`persistent-holder-set-or-tag` family of
`docs/registry-holder-consumer-coverage-26.1.json`. Work starts from fully
gated commit `112aa2a41`.

## Goal and Completion Standard

Every one of the 48 persistent HolderSet factories must receive one of two
source-backed outcomes:

- `complete` when the value is decoded or reconstructed by a bbb client
  consumer, with registry-aware direct and named-tag tests; or
- `not-applicable` when the factory belongs exclusively to server datapack,
  advancement, world-generation, trade, loot, or variant-selection behavior
  and the official client receives only the selected/network projection.

The fixed inventory is partitioned before implementation:

- 14 client component/predicate rows: `BlockPredicate`, the six concrete data
  component predicate HolderSets, and the seven item-component HolderSets;
- six synchronized `Enchantment.DIRECT_CODEC` rows: exclusive/supported/
  primary sets plus the `apply_mob_effect`, `explode`, and `summon_entity`
  effect HolderSets; and
- 28 server-only rows in advancement predicates, server variant selection,
  enchantment providers, villager trades, world generation, and loot
  functions.

Completion requires all 48 rows to leave `partial`; no row may be classified
by registry name alone. Direct resources and named tags must bind through the
same frozen revision that owns the enclosing persistent value. Missing tags,
unknown resources, malformed list members, custom insertion/reordering, tag
replacement, or configuration re-entry must never reinterpret an identity.

## Non-Goals

- The other persistent Holder/HolderSet/TagKey families and the two remaining
  network families (`Instrument` holder-or-inline and recipe `TagKey`) remain
  separate ledger work.
- bbb does not execute server-side advancement, loot, world-generation,
  enchantment entity-effect, trade-selection, or variant-selection gameplay.
- This slice does not reproduce Mojang's generic Codec/DynamicOps framework.
  It decodes the exact HolderSet scalar/list/tag shape from the already bounded
  canonical NBT tree.
- Enchantment effect validation does not imply client execution of
  `apply_mob_effect`, `explode`, or `summon_entity`; it only closes the
  synchronized registry identity boundary used while accepting the enclosing
  definition.
- No context-free caller may accept a nonempty persistent HolderSet by using a
  bootstrap ordinal, first entry, empty set, or unvalidated resource string.

## Owners and Frozen Boundaries

- `bbb-world::registries` owns a single persistent HolderSet NBT resolver. It
  normalizes resource locations, resolves compact scalar/list direct forms and
  `#tag` forms, and returns checked entries from one `FrozenRegistryAccess`.
- Dialog component and command-item-predicate decoding remain semantic
  consumers, but use the registry-owned resolver rather than maintaining a
  second persistent HolderSet parser. Existing context-free empty-direct
  support remains the sole explicit no-registry case.
- The synchronized enchantment materializer validates and retains the three
  top-level HolderSet identities. It also walks the bounded effect tree and
  validates the three polymorphic effect HolderSets wherever their exact
  `type` discriminator occurs. Candidate failure is atomic.
- Enchantment queries retain named-vs-direct identity and resolve named sets
  through the current tag revision, so `UpdateTags` cannot leave an expanded
  stale cache.
- `bbb-native` persistent item-component reconstruction continues to use
  `PersistentSnbtContext::registry_holder_set_snbt`; it never becomes a second
  registry owner.
- Coverage families are split only by the frozen source-path partitions above.
  The 28 `not-applicable` rows name no bbb consumer and carry Vanilla evidence
  for their server-only load/execution boundary.

## Vanilla 26.1 Evidence

`RegistryCodecs.homogeneousList` delegates to `HolderSetCodec`, whose
persistent shape is one resource id, a list of resource ids, or a `#tag`.
Direct values use the supplied registry-aware element codec; tags are legal
only when the registry key matches.

`RegistryDataLoader.SYNCHRONIZED_REGISTRIES` includes `ENCHANTMENT` with
`Enchantment.DIRECT_CODEC`, so its exclusive/supported/primary sets and nested
effect values cross configuration as registry-aware persistent data.
`RegistrySynchronization` packs only that synchronized list. The remaining
28 factories are reached only from server registries/codecs or server runtime
types; animal variant network codecs omit the persistent spawn-condition
checks and send the already selected presentation.

The six synchronized enchantment forms are exact:

- `exclusive_set` uses the enchantment registry;
- required `supported_items` and optional `primary_items` use the item
  registry;
- `ApplyMobEffect.to_apply`, optional `ExplodeEffect.immune_blocks`, and
  `SummonEntityEffect.entity` use mob-effect, block, and entity-type
  registries respectively.

These facts are verified from local sources under
`/home/zgy/Work/mc-code/sources/26.1` before each correction.

## Ordered Stages and Focused Tests

### A. Registry-owned persistent HolderSet resolver

Add the exact NBT scalar/list/tag resolver and typed errors to frozen registry
access. Migrate dialog component/predicate decoding to it, preserve the lone
context-free empty-direct rule, and reuse it for any existing registry
persistent-set decoder whose contract matches.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world persistent_holder_set
scripts/cargo-dev.sh test -p bbb-world dialog --lib
```

### B. Synchronized enchantment identity

Validate and retain exclusive/supported/primary shapes during candidate
materialization. Validate nested holder-bearing effect variants recursively,
publish a checked query, and prove custom order, tag form, missing resources,
tag replacement, and configuration failure/re-entry are revision-safe.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world enchantment --lib
scripts/cargo-dev.sh test -p bbb-world registry --lib
```

### C. Existing component consumers

Prove the 14 component/predicate call sites use the shared resolver through
dialog/command decoding or registry-aware native persistent reconstruction.
Add reverse structural guards where existing semantic tests are not enough;
do not duplicate component decoders merely for ledger evidence.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-world holder_set --lib
scripts/cargo-dev.sh test -p bbb-native holder_set
```

### D. Coverage classification and closeout

Split the broad family into the three exact source-path partitions, regenerate
the ledger, and assert the 48-row inventory, 20 complete rows, 28
not-applicable rows, and zero partial/deferred rows. Reverse tests pin the
registry resolver, synchronized enchantment validation, and absence of a
consumer on server-only rows.

Focused tests:

```text
scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check
scripts/cargo-dev.sh test -p bbb-protocol registry_holder_coverage
```

## Checkpoints, Failures, and Safe Stops

The contract, shared resolver, enchantment closure, and coverage closeout are
separate buildable commits. Run a full gate by the fourth commit, at the final
engineering HEAD, and before switching semantic domains. A focused failure in
candidate materialization triggers a full gate after the fix.

Safe stops are a clean contract commit; a shared resolver with all migrated
callers and old valid values unchanged; atomic enchantment validation without
ledger status changes; or a source-backed blocker documented against the exact
row. Failures are fixed forward and no coverage row changes status before its
consumer tests or server-only proof lands.
