# Vanilla 26.1 registry holder consumer inventory

The checked-in source of truth is
`crates/bbb-protocol/src/registry_holder_consumers.rs`. The bootstrap generator
prefilters vanilla Java sources, parses every candidate with the shared
Tree-sitter frontend, and records concrete factory calls rather than matching
Java source text.

The inventory currently contains 194 call sites across these semantic shapes:

- persistent named or inline `Holder` codecs;
- persistent direct-list or named-tag `HolderSet` codecs;
- network reference or inline `Holder` stream codecs;
- network direct-list or named-tag `HolderSet` stream codecs;
- persistent, hashed, and network `TagKey` codecs;
- built-in `holderByNameCodec` calls.

Each row retains source path, line and column, owning Java type, factory, shape,
the original registry expression, and a resource id when the expression names a
concrete `Registries` or `BuiltInRegistries` constant. Generic helper calls are
kept with no resource id instead of being discarded. `RegistryPacket` exposes
the number of concrete vanilla call sites tied to its registry for diagnostics.

The generated
`docs/registry-holder-consumer-coverage-26.1.json` ledger maps all 49 network
factory calls to six checked families. Reference/inline, reference-only,
holder-set/tag, and tag-only calls name their current bbb decoder and world
resolver evidence and remain `partial` until registry context and custom-reorder
tests are closed. The two typed debug holder calls are narrowly `deferred` until
raw debug payload retention is replaced by the `DEBUG_SUBSCRIPTION` value
codecs. The generic `holderRegistry` delegation inside vanilla `holderSet` is
`not-applicable` as a distinct consumer because every concrete holder-set call
is inventoried separately.

The remaining ledger work is to classify the 145 persistent Codec call sites
with the same evidence standard and close each applicable registry family's
context threading and custom-reorder tests. Neither catalog presence nor a raw
numeric-id decoder is sufficient evidence for `complete`.

Regenerate and verify the inventory through the existing bootstrap catalog
generator. The authoritative pre-commit gate remains the one in `goal.md`.
