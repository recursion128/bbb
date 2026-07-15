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
`docs/registry-holder-consumer-coverage-26.1.json` ledger maps all 194 calls to
18 checked families. The four network shapes and seven persistent Codec shapes
name their current bbb resource-id or tag resolver surface. They remain
`partial` until the owning decoder/encoder receives registry context and the
registry family has custom-reorder tests; this accounts for 178 rows. The
`sound_event` reference/direct stream codec is `complete`: vanilla owns it in
the client built-in layer, and bbb's generated static order, direct holder
payload, world ownership, and audio runtime resolution have cross-layer tests.
The synchronized chicken, cow, and pig variant reference holders are also
`complete`: server order selects a typed `NETWORK_CODEC` presentation, and its
model plus adult/baby custom textures reach the entity renderer atomically.
The three villager type/profession reference calls are `complete` against their
generated client built-in order across entity metadata, item components,
item-model predicates, and rendering; configuration registry injection is
explicitly tested not to remap them.
The synchronized unit-valued `world_clock` stream holder is `complete`: SetTime
retains the server holder ids, while the active dimension's default clock
resource id resolves through the session order before merge, tick, and day-time
projection; a reversed custom order covers that identity boundary.
The synchronized cat, chicken, cow, pig, and wolf sound-variant stream holders
are `complete`: entity metadata ids resolve in each server registry order, all
five codec shapes retain their required named/direct SoundEvent holders, and
adult/baby death sounds reach positioned audio with malformed payloads closed.
The two typed debug holder calls are narrowly `deferred` until raw debug payload
retention is replaced by the `DEBUG_SUBSCRIPTION` value codecs. The generic
`holderRegistry` delegation inside vanilla `holderSet` is `not-applicable` as a
distinct consumer because every concrete holder-set call is inventoried
separately.

Neither catalog presence, a raw numeric-id decoder, nor the existence of the
generic resolver API is sufficient evidence for `complete`.

Regenerate and verify the inventory through the existing bootstrap catalog
generator. The authoritative pre-commit gate remains the one in `goal.md`.
