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

This source inventory does not claim bbb consumer parity. The remaining ledger
work is to map every row or shared family to the world resolver consumer that
implements it, a vanilla-backed not-applicable decision, or a narrow deferred
condition, then close each registry family's custom-reorder tests.

Regenerate and verify the inventory through the existing bootstrap catalog
generator. The authoritative pre-commit gate remains the one in `goal.md`.
