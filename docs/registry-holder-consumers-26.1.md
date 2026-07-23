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
42 checked families. Coverage selectors may narrow a shared vanilla source by
concrete resource id without classifying its unrelated registry calls. All rows
are closed: 107 client-observable calls are `complete`, and 87 generic helper or
server-owned persistent calls are `not-applicable` to the native client. The
single network `TagKey` call belongs to `TagSlotDisplay`: protocol retains its
normalized item-tag identity, and native recipe presentation resolves ordered
members from the current `UpdateTags` revision on every read. Replacement and
removal are covered without a parallel tag cache. The remaining persistent
TagKey/hashed-tag, custom HolderSet, and named-or-inline HolderSet rows are
generic codec delegation or server advancement, block bootstrap, recipe data,
worldgen, and loot inputs; Vanilla's client paths use their separately
inventoried stream/network codecs where applicable. The
`sound_event` reference/direct stream codec is `complete`: vanilla owns it in
the client built-in layer, and bbb's generated static order, direct holder
payload, world ownership, and audio runtime resolution have cross-layer tests.
The synchronized chicken, cow, and pig variant reference holders are also
`complete`: server order selects a typed `NETWORK_CODEC` presentation, and its
model plus adult/baby custom textures reach the entity renderer atomically.
The synchronized cat-variant reference holder is `complete` by the same
server-order boundary: both age-specific `ResourceTexture` fields are decoded,
uploaded, and selected by the entity renderer, while stubs and failed uploads
retain the checked built-in identity fallback.
The synchronized frog-variant reference holder is likewise `complete`: its
single `ResourceTexture` is decoded in server holder order and reaches the
entity instance only after the custom image upload succeeds.
The synchronized zombie-nautilus reference holder is `complete`: its
`ModelAndTexture` normal/warm selector and resource asset are projected in
server order, and geometry plus texture switch together only after upload.
The synchronized wolf-variant reference holder is `complete`: both adult and
baby `AssetInfo` compounds project their wild, tame, and angry resources in
server order; all six are uploaded, and entity state selects the ready texture
with vanilla's tame-over-angry priority.
The three villager type/profession reference calls are `complete` against their
generated client built-in order across entity metadata, item components,
item-model predicates, and rendering; configuration registry injection is
explicitly tested not to remap them.
The synchronized unit-valued `world_clock` stream holder is `complete`: SetTime
retains the server holder ids, while the active dimension's default clock
resource id resolves through the session order before merge, tick, and day-time
projection; a reversed custom order covers that identity boundary.
The synchronized `dimension_type` reference holder is `complete`: Login and
Respawn retain the `CommonPlayerSpawnInfo` numeric id, and level creation
resolves it through the current server order before applying identity, shape,
lighting, skybox, ambient light, clock, and environment projections. The
persistent direct holder codec is server-only. The shared direct/network element
codec's `infiniburn` hashed tag is complete through the atomic typed revision:
its exact tag identity reaches the active level alongside every scalar and
monster-light provider field, while member order stays in the same frozen
block-tag revision for registry queries.
The synchronized `damage_type` reference holder is `complete`: DamageEvent and
component values retain the current server order, custom network elements are
strictly projected across the five direct-codec fields, and generated known-pack
effects plus custom reordered effects select the six vanilla player hurt sounds
through the native audio command path. The same source id controls wolf-armor
bypass tags, while generated runtime-class policies cover every fixed living
EntityType's hurt sound and volume without requiring a projected effect for
non-player sounds. Persistent codec and holder-set calls remain separate
classified rows.
The `map_decoration_type` reference holder is `complete` against its immutable
client built-in registry: MapItemData keeps the numeric holder, the generated
35-entry order supplies sprite and item-frame visibility, and both first-person
and item-frame map surfaces consume it. Configuration registry injection is
explicitly tested not to remap that order; the persistent named codec remains a
separate classified row.
The two item reference calls are `complete` against the immutable 1506-entry
client built-in registry. All direct `Item.STREAM_CODEC` owners validate ids at
their packet boundary, item-stack templates reject air, serverbound hashed stacks
reject identities outside that same order, and a reversed RegistryData injection
cannot remap the world lookup. Persistent item codecs and holder sets remain
separate rows.
The mob-effect reference call is `complete` against its immutable 40-entry
client built-in registry. Update/Remove, beacon encoding, nested effect instances,
and suspicious-stew entries validate the generated order at their wire boundary;
entity state and its movement, lightmap, HUD, tooltip, and item-component consumers
retain that identity, while reversed RegistryData cannot remap it. Persistent
mob-effect codecs and holder sets remain separate rows.
The attribute reference call is `complete` against its immutable 35-entry
client built-in registry and connection RegistryAccess view. UpdateAttributes
retains the wire holder until the world boundary, resolves the resource through
the frozen session view, and validates membership against the generated target
entity `AttributeSupplier`; item attribute modifiers keep strict built-in
validation in their context-free component decoder. Scale, movement, tooltip,
and item-model consumers retain canonical generated ids. Custom holder order,
session reset, entity-id reuse, and retained publication isolation are covered;
persistent attribute codecs and holder sets remain separate rows.
The potion reference call is `complete` against its immutable 46-entry client
built-in registry. PotionContents validates its optional holder at the wire
boundary, world lookup preserves the generated identity in both directions, and
tooltip, model-predicate, and persistent-input consumers share the generated
resource ids and effects. Reversed RegistryData cannot remap the identity;
persistent potion codecs and holder sets remain separate rows.
The synchronized enchantment reference holder is `complete`: both enchantments
component maps reject negative references while retaining nonnegative server
ids, and custom reordered definitions resolve through the session holder order
for tooltip text, attribute effects, item-model predicates, and crossbow charge
effects. The remaining effect-component semantics and persistent enchantment
codecs/holder sets stay tracked separately.
The synchronized dialog reference/direct holder is `complete`: ShowDialog
strictly preserves a nonnegative server reference or bounded direct compound;
world resolution distinguishes reordered definitions, stubs, malformed data,
and unknown ids, and both direct and custom referenced dialogs reach the native
screen and nested-action lifecycle. Persistent dialog codecs and holder sets
remain separate rows.
All four banner-pattern holder shapes are `complete`: named/direct layer holders
resolve through synchronized order and typed direct fields, while
`provides_banner_patterns` keeps an explicit holder list or named tag through
network and persistent forms. Tooltip, block-entity, SNBT, and runtime atlas
consumers share that identity; custom sprites use the `banner_patterns` atlas and
missingno rather than a bootstrap ordinal. The deferred no-depth-write render
pipeline is a separate presentation gap, not a holder-identity gap.
Both painting-variant holder shapes are `complete`: the network stream codec
retains named or inline synchronized values, while the persistent
`RegistryFixedCodec` accepts named references only. Current-session identity
drives typed tooltip text, entity bounds/direction, per-tile light, and
front/back/boundary geometry from the `paintings` atlas; malformed or unknown
holders fail closed, and a direct network holder is never fabricated into a
persistent inline value.
The synchronized entity-metadata variant holders are `complete` at the holder
boundary: before canonical mutation, each wire id is stored together with its
exact resource id, source, and frozen registry revision. Visual catalogs are
shared below world/renderer, sound and painting catalogs remain single-owned by
world, and stale revisions fail closed instead of reinterpreting an ordinal.
The cat, chicken, cow, pig, and wolf sound-variant stream holders retain all
required named/direct SoundEvent forms; adult/baby death sounds reach
positioned audio with malformed payloads closed. This holder completion does
not mark the wider entity-family render/event matrix complete.
The two typed debug holder calls are narrowly `deferred` until raw debug payload
retention is replaced by the `DEBUG_SUBSCRIPTION` value codecs. The generic
`holderRegistry` delegation inside vanilla `holderSet` is `not-applicable` as a
distinct consumer because every concrete holder-set call is inventoried
separately.

Neither catalog presence, a raw numeric-id decoder, nor the existence of the
generic resolver API is sufficient evidence for `complete`.

Regenerate and verify the inventory through the existing bootstrap catalog
generator. The authoritative pre-commit gate remains the one in `goal.md`.
