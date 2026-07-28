# Configuration registry packet coverage closure

## Goal and completion contract

Close the remaining coverage cells for configuration
`ClientboundRegistryDataPacket` and `ClientboundUpdateTagsPacket`. Registry
contents and tags received during configuration must remain isolated from the
active session, materialize through one candidate, and become visible to native
consumers together at the Configuration-to-Play transition. Invalid candidates
must retain the previous active revision and request disconnect rather than
publishing a mixed registry/tag generation.

Completion means both packet rows are fully `complete`, one native integration
test proves pre-transition isolation plus post-transition registry and tag
presentation, generated packet coverage agrees with the override source, and
the full gate passes at the milestone head.

## Scope and non-goals

- `bbb-protocol` continues to own bounded wire decoding for registry contents
  and tag maps.
- `bbb-net` continues to own configuration-state packet dispatch and the state
  transition event.
- `bbb-world` remains the sole owner of the pending/active registry candidate,
  typed projections, tag materialization, validation, and revision counters.
- `bbb-native` continues to synchronize runtime owners only from the active
  world revision.

This slice does not add another registry cache, expose pending state to render
code, broaden Mojang Codec replication, implement new registry value families,
or change play-state `UpdateTags` replacement semantics. Registry-holder and tag
consumer classification remains owned by the separate 194-row ledger.

## Vanilla and integration contract

- `ClientConfigurationPacketListenerImpl.handleRegistryData` calls
  `RegistryDataCollector.appendContents`, and `handleUpdateTags` calls
  `appendTags`; neither mutates the active play registry directly.
- `handleConfigurationFinished` invokes
  `RegistryDataCollector.collectGameRegistries` before installing the play
  protocol and sending the finish acknowledgement.
- `RegistryDataCollector.collectGameRegistries` joins synchronized contents and
  their tags, resolves eligible static-registry tags, and loads the frozen
  registry access as one result.
- bbb's `StateChanged(Configuration)` starts a pending collection;
  `RegistryData` and `UpdateTags` append only to it. `StateChanged(Play)` builds
  and validates one candidate, atomically installs contents, typed values,
  tags, and registry-dependent item defaults, then synchronizes native audio
  registries. Candidate failure queues disconnect and leaves the old active
  revision intact.
- The integration proof uses synchronized jukebox-song identity as the
  registry-backed native consumer and `minecraft:piglin_loved` as the tag-backed
  entity-presentation consumer. Before Play both must read the old active
  revision; after Play both must read the new revision.

Authoritative sources are
`net/minecraft/client/multiplayer/ClientConfigurationPacketListenerImpl.java`,
`net/minecraft/client/multiplayer/RegistryDataCollector.java`,
`net/minecraft/network/protocol/configuration/ClientboundRegistryDataPacket.java`,
and `net/minecraft/network/protocol/common/ClientboundUpdateTagsPacket.java`
under the local vanilla 26.1 source tree.

## Serial stages and verification

1. Extend the native configuration-transition regression to include a pending
   item tag and assert both registry-backed audio identity and tag-backed piglin
   pose before and after Play. Focused test: the named `bbb-native` event test,
   plus non-test `bbb-native` check.
2. Mark the three remaining packet-coverage cells complete, add a reverse guard
   for the two fully closed rows, regenerate the packet ledger, and narrow the
   unsupported protocol TODO to remaining play families. Focused tests: packet
   catalog generator tests/check and `bbb-protocol` packet coverage tests plus
   non-test check.
3. Append the milestone index entry, review the integrated diff, and run
   `scripts/cargo-dev.sh gate` followed by `scripts/playable-gate.sh auto`.

The fully gated `ae5ef9151` is the pre-start checkpoint. Each stage is a clean,
buildable commit. A pre/post-transition mismatch stops in stage 1; a ledger or
generator mismatch stops in stage 2. Failures are fixed forward without
rewriting history.
