# Entity Metadata Rich Values and Registry Identity

This document freezes the high-risk engineering contract for the remaining
`ClientboundSetEntityDataPacket` value-preservation foundation. The generated
entity accessor schema already validates every data id and serializer before
mutation. This slice completes the value boundary beneath family-specific
rendering, animation, and event milestones; it does not claim that all 157
entity families are otherwise complete.

## Semantic Goal and Completion

One decoded metadata update must preserve the same client-observable value that
Vanilla 26.1 assigns to `SynchedEntityData`, while keeping `WorldStore` as the
only mutable canonical owner. Completion requires:

- trusted `Component` and optional-component serializers retain their exact
  validated network-NBT tree, inherited styled runs, and plain-text projection
  as one structured value;
- metadata `ItemStack` values use the same complete bounded data-component
  decoder as inventory/equipment, and every component payload needed by a
  metadata consumer is retained rather than replaced by only a count or text;
- every metadata holder serializer is resolved against the frozen registry of
  the current session before canonical mutation, retaining the wire holder id,
  exact resource id, registry source, and registry revision together;
- unknown holder ids or invalid registry-sensitive nested Component payloads
  reject the whole metadata packet before mutation and produce bounded apply
  diagnostics;
- renderer, debug, control, and family helpers read the canonical structured
  value and do not independently reinterpret a holder id against another
  generation; and
- a retained publication, registry reload/reconfiguration, reconnect, level
  reset, entity removal, and protocol-id reuse cannot alter or revive an older
  metadata value.

The existing plain text remains an explicit projection for callers that only
need glyph-independent text. Rich state is serializable canonical data, not a
renderer cache. Render and control still consume the same exact world `Arc` and
publication generation.

## Non-goals

- Do not change packet ids, field order, tick cadence, input, interpolation,
  rendering order, effect ordering, or server authority.
- Do not introduce a second metadata schema, a parallel holder-membership map,
  a second canonical entity world, global registry state, `unsafe`, or a shared
  mutable `WorldStore`.
- Do not implement missing family models, animations, particles, audio, AI,
  movement, or debug fields merely because their metadata accessor exists.
- Do not copy Mojang's complete `Codec` or Java class hierarchy. Decode only
  the typed values carried by current wire and concrete consumers.
- Do not describe the wider SetEntityData/entity-family ledger entry as closed
  while family-specific consumers remain partial.

## Authoritative Vanilla 26.1 Behavior

- `EntityDataSerializers.COMPONENT` and `OPTIONAL_COMPONENT` use
  `ComponentSerialization.TRUSTED_STREAM_CODEC` and its optional wrapper.
- `EntityDataSerializers.ITEM_STACK` uses `ItemStack.OPTIONAL_STREAM_CODEC` and
  copies the mutable stack value when a data value is packed.
- dynamic entity variant serializers use their registry-aware holder stream
  codecs over the connection's `RegistryFriendlyByteBuf`.
- `ClientboundSetEntityDataPacket` reads `(data id, serializer id, value)` rows
  until byte `255` and `ClientPacketListener.handleSetEntityData` assigns the
  decoded list on the client thread.
- `SynchedEntityData.assignValues` checks serializer identity before assignment,
  invokes per-accessor callbacks in row order, then invokes the batch callback.

The local sources of record are:

- `net/minecraft/network/syncher/EntityDataSerializers.java`;
- `net/minecraft/network/syncher/SynchedEntityData.java`;
- `net/minecraft/network/protocol/game/ClientboundSetEntityDataPacket.java`;
- `net/minecraft/client/multiplayer/ClientPacketListener.java`;
- `net/minecraft/network/chat/ComponentSerialization.java`; and
- `net/minecraft/world/item/ItemStack.java` plus `DataComponentPatch.java`.

## Ownership and Representation

`bbb-protocol` owns bounded wire decoding and a structured rich-component value.
It preserves exact NBT and semantic projections but does not own a session
registry. Raw holder ids therefore cross the packet boundary without applying a
built-in ordinal.

`bbb-world` owns the frozen session registry and canonical entity metadata. Its
SetEntityData preflight validates the generated accessor schema, validates
registry-sensitive nested Component payloads, and resolves holder ids into one
value-local identity before any ECS or callback mutation. The stored identity
includes the registry access revision so stale or mixed-generation resolution
is mechanically visible. No side table is permitted.

`bbb-native` and `bbb-renderer` consume the structured value or narrow world
queries. They may use the stored resource identity directly, but may not resolve
the raw id against a separately loaded registry. Control serialization remains
a derived view of the same publication.

The generated cat, wolf, temperature-animal, zombie-nautilus, villager-type,
and villager-profession identity catalogs live in `bbb-render-types`, below both
world and renderer. The generated animal sound identity catalogs remain in
`bbb-world`, where both registry fallback and audio consume them. Consequently
the frozen registry's exact Vanilla fallback for all 12 generic metadata holder
serializers, named painting variants, villager type, and villager profession is
backed by the same generated catalog as its consumer; there is no second ordinal
table.

The internal serde shape for a metadata holder is now `{holder_id, identity}`.
`identity` contains registry/resource/source/revision and is absent only between
protocol decode and world preflight. This changes repository snapshot JSON, not
the Minecraft wire shape. World preflight always overwrites any caller-supplied
identity from the current frozen registry before canonical insertion. Dynamic
presentation/audio consumers additionally require the stored revision to equal
the immutable world's current registry revision; debug/save projection reads the
stored resource directly and never reinterprets the ordinal.

Heap-heavy component trees and item component patches retain the existing
per-component immutable publication owner. A metadata mutation disconnects only
the affected entity payload; it never mutates a retained generation and does
not introduce whole-`EntityStore` COW.

## ItemStack Decoder Audit

Vanilla's metadata `ITEM_STACK` serializer does not define a distinct stack
shape: its `codec()` returns `ItemStack.OPTIONAL_STREAM_CODEC`. The bbb packet
path now has executable evidence that metadata calls the same
`inventory::decode_item_stack_summary` used by inventory and equipment. A
source guard rejects a metadata-only component decoder, while a nested wire
fixture feeds the same bytes through both entry points and compares custom NBT,
a rich name, model identity, foil state, removals, and a recursively patched
`use_remainder` stack.

The generated accessor schema identifies the current metadata owners: dropped
items, item displays, item/glow frames, firework rockets, ominous item
spawners, eyes of ender, and throwable/hurting item projectiles. Implemented
world consumers clone the complete `ItemStackSummary`; specialized map and
firework projections then read from that same value. The shared component
catalog retains every field those consumers currently inspect, including
model/custom-model selection, profile/trim/variant data, map id, foil and
enchantments, firework explosions, and recursively decoded remainder,
projectile, bundle, and container stacks. The audit found no consumed payload
routed through a skip or text/count-only metadata projection.

This is intentionally a consumer-scoped claim, not a claim that the summary is
Mojang's complete mutable `PatchedDataComponentMap`. Nonconsumer-only wire
values may remain represented by presence/type evidence until a concrete
native consumer exists. They still use the one shared decoder, so metadata
cannot silently diverge from inventory or equipment. A retained-generation
test mutates outer and nested component NBT and proves both the canonical item
query and raw metadata in generation A stay unchanged while generation B sees
the new stack.

## Serial Phases and Gates

1. **Contract and Component value.** Add exact Component NBT/styled/plain
   preservation, migrate metadata readers, validate nested owner-sensitive
   payloads at world preflight, and add protocol/world/native semantic tests.
   Commit normally and run the first milestone full gate.
2. **Holder identity.** Generate or declare the exact serializer-to-registry
   mapping once, bind holder ids to resource/source/revision in the canonical
   value, migrate variant/debug/render consumers, and test custom insertion,
   reorder, missing holder, reconnect, reload, and id reuse. Commit normally
   and run the second milestone full gate.
3. **ItemStack audit.** Compare every metadata ItemStack path with
   `ItemStack.OPTIONAL_STREAM_CODEC` and the shared data-component catalog;
   preserve any concrete payload found to be dropped, add nested-component and
   retained-publication tests, and reject a separate metadata-only decoder.
   Commit normally.
4. **Coverage and closeout.** Update packet/registry/entity coverage evidence,
   keep only family-specific residual TODOs, update architecture/archive, and
   run focused tests, formatting, diff/docs checks, and a complete diff review.
   Commit and run the final full gate.
5. Build `bbb-native` from the delivered HEAD in `/tmp/bbb-target-main`, verify
   its timestamp against HEAD, run the explicit Weston-backed live Vanilla 26.1
   playable gate, then run the policy `auto` gate.

Focused validation covers `bbb-protocol` entity packet codecs, `bbb-world`
metadata/registry/publication tests, and `bbb-native` entity scene/debug/control
tests. Every phase is buildable and reviewable. No commit is amended or rebased.

## Failure Localization and Safe Stops

Protocol failures stop in rich Component decoding; world failures stop in the
atomic preflight or registry binding; native failures stop in a migrated
consumer; publication failures stop in retained-generation tests. Once a phase
changes a public value representation, there is no safe stop until all callers
compile, focused semantic tests pass, and that phase's full gate is green.

Malformed rich payloads and missing holder ids fail the packet atomically. They
do not fall back to a bootstrap ordinal, partially apply preceding values, or
publish mixed old/new metadata. Any gate failure is fixed forward by a normal
commit.

## Phase Status

- Phase 1 (`09b6402d1`) preserves exact rich Component NBT/styled/plain values;
  its milestone full gate passed at that commit.
- Phase 2 binds generic holders, VillagerData holders, and named painting
  holders atomically. Missing ids reject the packet, retained generations keep
  their original identity, protocol-id reuse binds the current session, and
  renderer/debug/audio reject cross-revision ordinal reuse. The generated
  visual identity catalogs moved to `bbb-render-types` so the frozen fallback
  and renderer share one owner.
- Phase 3 audits every generated metadata ItemStack owner against Vanilla's
  optional stack codec and bbb's shared component catalog. Metadata,
  inventory, and equipment have one bounded decoder; nested patch and retained
  publication tests cover the full-value path, and no concrete current
  metadata consumer payload was found on a lossy branch.
