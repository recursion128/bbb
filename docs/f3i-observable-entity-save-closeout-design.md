# Shift-local F3+I observable entity save closeout

## Goal and completion criteria

Close the remaining Shift-local F3+I entity `saveWithoutId` gap without inventing
server-private state. The slice is complete when every residual named in the
`HUD overlay and screen render surfaces` ledger entry is one of:

- projected from synchronized metadata, spawn data, equipment/components,
  client-owned runtime, or a protocol-backed registry reference;
- omitted with a focused Vanilla proof that the client does not observe the
  value or that the persistent codec cannot reconstruct the network holder; or
- represented by an exact bounded persistent value already retained by the
  protocol summary.

The final recreate command must continue to use the one existing
`debug_local_entity_pretty_snbt` dispatch and persistent component encoder
registry. Focused tests must prove exact SNBT keys, default omission, registry
identity, nested item/component shape, and the absence of unobserved fields.

## Non-goals

- Do not synthesize server-private mutable owner/target/inventory/timer/RNG
  state from Java constructor defaults. A constructor value is projectable only
  with a focused proof that the external client owns it and no client-observable
  path can change it.
- Do not change ordinary F3+I: it remains server-tag-query-authoritative.
- Do not add an integrated-server boundary or use server sources at runtime.
- Do not create a second entity save dispatcher, component encoder table, or
  hand-maintained registry ordinal table.
- Do not broaden this slice into entity rendering, interaction, or general NBT
  editing.

## Ownership and integration contract

- `bbb-protocol` owns exact bounded network retention for values that are
  present on the wire. `DataComponentPatchSummary` and generated entity
  metadata descriptors remain the only protocol summaries consumed here.
- `bbb-world` owns synchronized entity state and registry resolution. New
  observable runtime state is added only when an existing packet/metadata/tick
  path can update it faithfully.
- `bbb-native` owns the Shift-local projection in
  `input.rs::debug_local_entity_pretty_snbt` and the sorted
  `input/component_patch.rs::PERSISTENT_SNBT_ENCODERS` registry.
- Persistent holder ids resolve through the current frozen session registry;
  fixed Vanilla catalogs are used only when that registry has no server
  contents. Unknown ids omit the containing value rather than guessing.
- Nested item stacks use the existing depth bound and require an exact
  persistent component patch; partial nested values never produce misleading
  SNBT.

## Vanilla basis

- `KeyboardHandler.copyRecreateCommand` calls `Entity.saveWithoutId` only for
  Shift-local capture; ordinary capture sends a server tag query.
- `Entity.saveWithoutId` writes the base entity state and then invokes the
  concrete `addAdditionalSaveData` inheritance chain.
- `LivingEntity.addAdditionalSaveData`, `AbstractArrow.addAdditionalSaveData`,
  the minecart hierarchy, and container entity implementations define the
  remaining key names, default omission, codec shape, and parent-before-child
  ordering.
- Persistent item component shape comes from `DataComponents` and each
  registered component `Codec`; network-only components have no persistent
  projection.

Authoritative sources are under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/`.

## LivingEntity parent projection contract

The Living parent stage uses the inherited save order from
`LivingEntity.addAdditionalSaveData` and projects only state retained by the
native client:

- synchronized health, fall-flying flags, sleeping position, and Player
  absorption use generated metadata accessors;
- client-owned hurt/death counters use the canonical animation state and keep
  Vanilla's short-tag representation;
- attribute snapshots emit only `id` and `base`: the client packet handler
  installs every wire modifier with `addTransientModifier`, while
  `AttributeInstance.pack` serializes only permanent modifiers;
- active mob effects use the retained effect holder, duration, amplifier, and
  display flags; `forceAddEffect` replaces the client instance, so the
  unsynchronized hidden-effect chain remains absent;
- equipment uses `EntityEquipment.CODEC` slot names, filters empty stacks, and
  delegates each non-empty value to the existing persistent item-stack encoder.
  An unknown item or unsupported component omits the complete equipment value
  instead of emitting a partial map.

`Brain.Packed` is deliberately omitted. It is not generally an empty client
constant: Copper Golem installs a randomized transport cooldown in its
constructor, Allay interaction can install `LIKED_PLAYER`, and Villager wake
transitions install `LAST_WOKEN`. Those memories need their own client-owned
runtime state before they can be encoded. Unretained last-attacker references,
impulse context, and locator-icon state are likewise not synthesized here.

Registry-backed entity variants use the identity frozen onto each synchronized
metadata value, including server-defined resources. Tropical fish keep their
wire-packed metadata, but persistent projection follows
`TropicalFish.Variant.CODEC`: decode the sparse pattern and both dye ids through
their Vanilla fallback maps, then repack the normalized value.

Packet-created non-living entities also keep constructor state that Vanilla
always saves and the client can know exactly:

- FallingBlock keeps `DropItem`, `HurtEntities`, fall-damage, and `CancelDrop`
  defaults while its client-owned `Time` advances;
- Item and ExperienceOrb keep health, pickup/count defaults, and client-owned
  ages; ExperienceOrb skips its complete first client tick before age advances;
- FireworkRocket keeps zero `LifeTime`, its default rocket stack, and the
  synchronized `ShotAtAngle`, while `Life` advances locally;
- EyeOfEnder saves its default eye stack until synchronized item metadata
  replaces it; OminousItemSpawner saves the packet-constructor's zero delay
  because only the server factory randomizes that field.

## Serial stages and focused checks

1. **Persistent value audit.** Mechanically compare all 26.1 persistent data
   component descriptors with `PERSISTENT_SNBT_ENCODERS`; close exact rich
   Component and other codec-backed residuals, plus item/block-state values
   used by entity save paths.
   - `scripts/cargo-dev.sh test -p bbb-protocol data_components`
   - `scripts/cargo-dev.sh test -p bbb-native input::component_patch::tests`
   - `scripts/cargo-dev.sh test -p bbb-native input::tests::item_stack_snbt`
2. **Entity and Living parent state.** Audit the generated entity handler
   dispatch against each concrete Vanilla save chain; project remaining
   synchronized constructor/default, health/equipment/effect/attribute,
   packed-variant, projectile, falling-block, item-frame, and vibration fields.
   - `scripts/cargo-dev.sh test -p bbb-world entities`
   - `scripts/cargo-dev.sh test -p bbb-native input::tests`
3. **Observable inventories and specialized minecarts.** Project only
   protocol-backed non-empty container/loot data plus furnace, spawner, and
   command-block minecart fields whose client value is exact.
   - `scripts/cargo-dev.sh test -p bbb-world inventory`
   - `scripts/cargo-dev.sh test -p bbb-native input::tests`
4. **Reverse audit and closeout.** Add/refresh structural assertions that every
   generated entity save handler and every persistent component descriptor has
   one explicit decision. Remove the closed ledger text and archive this
   contract with a short milestone index entry.

## Checkpoints and failure isolation

Each stage ends in a clean, reviewable commit after focused tests, format,
diff-check, docs lint, and full diff review. Any stage that changes protocol
decode/encode or generated catalogs runs a full gate immediately; otherwise a
full gate runs at four commits or before crossing into another semantic domain.
The final closeout always runs `scripts/cargo-dev.sh gate`.

The safe stopping point is a clean committed stage whose applicable full gate
passes. A partially decoded value, a partially wired world field, or an
uncommitted ledger deletion is not a safe stopping point. Failures are isolated
first by component descriptor, entity inheritance family, or minecart/container
family before using `batch-gate`.
