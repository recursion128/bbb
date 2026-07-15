# Vanilla 26.1 RegistryData inventory

The checked-in source of truth is
`crates/bbb-protocol/src/registry_data_types.rs`. It is generated from vanilla
`RegistryDataLoader.java` and `Registries.java`; do not copy the lists into this
document or maintain them by hand.

The generated inventory preserves three distinct loader lists:

- 45 world-generation registry definitions and their direct codecs;
- the one dimension registry definition;
- 28 configuration-synchronized registry definitions, including their network
  versus direct codec choice and optional validator expression.

`WorldStore` annotates every collected registry packet with membership in the
configuration-synchronized list. This makes the current permissive collection
visible without confusing static registries such as `minecraft:item` or
`minecraft:sound_event` with registries whose elements vanilla accepts through
`ClientboundRegistryDataPacket`.

Regenerate and verify the inventory through the existing bootstrap catalog
generator. The authoritative pre-commit gate remains the one in `goal.md`.

Remaining lifecycle work is tracked in `docs/unsupported-features.md`: production
must use this classification to ignore non-synchronized element packets, enforce
the declared validators when typed contents are available, and finish the
consumer/holder inventory.
